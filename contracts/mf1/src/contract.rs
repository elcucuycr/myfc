use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, IbcMsg
};
use crate::error::ContractError;
use crate::msg::{InstantiateMsg, QueryMsg, ExecuteMsg};
use crate::state::{CHAIN_ID, EXPECTED_TX_ID, MF_MAP, MF_VOTE_MAP, PENDING_TX_LIST, MY_LOGS};

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    MY_LOGS.save(deps.storage, &"start:".to_string())?;

    CHAIN_ID.save(deps.storage, &msg.chain_id)?;
    PENDING_TX_LIST.save(deps.storage, &Vec::new())?;
    EXPECTED_TX_ID.save(deps.storage, &1)?;
    MF_MAP.save(deps.storage, 0, &vec![Some(msg.original_value)])?;
    MF_VOTE_MAP.save(deps.storage, 0, &true)?;
    Ok(Response::new()
    .add_attribute("method", "instantiate")
    .add_attribute("initiated_chain", msg.chain_id.to_string())
    .add_attribute("original_value", msg.original_value.to_string()))
}

/* QUERY */
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg {
        Multifuture{ tx_id } => to_json_binary(&query::multifuture(deps, tx_id)?),
        MyLogs{} => to_json_binary(&query::my_logs(deps)?),
    }
}

mod query {
    use crate::msg::{MultifutureResp, MyLogsResp};

    use super::*;

    pub fn multifuture(deps: Deps, tx_id: u32) -> StdResult<MultifutureResp> {
        let mf = MF_MAP
        .load(deps.storage, tx_id)?
        .into_iter()
        .map(|i|{
            match i {
                None=>"None".to_string(),
                Some(v)=>v.to_string(),
            }
        })
        .collect::<Vec<String>>();
        Ok(MultifutureResp{futures: mf})
    }

    pub fn my_logs(deps: Deps) -> StdResult<MyLogsResp> {
        let logs = MY_LOGS.load(deps.storage)?;
        Ok(MyLogsResp{logs})
    }
}

/* EXECUTION */
pub fn execute(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    use ExecuteMsg::*;

    match msg {
        ExecuteTx{ fcross_tx } => exec::execute_tx(deps, &env, &fcross_tx),
        FinalizeTx{ instruction } => {
            let (attrs, msgs) = exec::finalize_tx(&mut deps, &env, &instruction)?;
            Ok(Response::new().add_attributes(attrs).add_messages(msgs))
        },
    }
}

pub mod exec {
    use crate::{error::ContractError, msg::{FcrossTx, Instruction}, state::{MAX_PENDING_LEN, MY_CHANNEL}, utils};

    use super::*;
    use crate::msg::{Operation, Vote};

    #[derive(Debug, Clone, Copy)]
    pub enum ExecutionStatus {
        Success,
        Failure,
        Uncertainty,
    }

    pub fn check_execution_stautus(values: &Vec<Option<i64>>) -> ExecutionStatus{
        let has_some = values.iter().any(|v| v.is_some());
        let has_none = values.iter().any(|v| v.is_none());
        match (has_some, has_none) {
            (true, false) => ExecutionStatus::Success,
            (false, true) => ExecutionStatus::Failure,
            (true, true) => ExecutionStatus::Uncertainty,
            _ => unreachable!(),
        }
    }

    pub fn give_vote(tx_id: u32, chain_id: u16, status: ExecutionStatus, channel_id: String, env: &Env) -> StdResult<IbcMsg>{
        // must eliminate case ExecutionStatus::Uncertainty before entering the function
        let my_vote = Vote{
            tx_id,
            chain_id,
            success: match status {
                ExecutionStatus::Success=>true,
                ExecutionStatus::Failure=>false,
                _=>unreachable!(),
            },
        };
        let msg = IbcMsg::SendPacket {
            channel_id,
            data: to_json_binary(&my_vote)?,
            timeout: utils::get_timeout(env),
        };
        Ok(msg)
    }

    pub fn execute_tx(
        deps: DepsMut,
        env: &Env,
        tx: &FcrossTx,
    ) -> Result<Response, ContractError> {
        // pre-execution check
        let expected = EXPECTED_TX_ID.load(deps.storage)?;
        if tx.tx_id != expected{
            return Err(ContractError::MismatchedExecutionTxId { sent_id: tx.tx_id, expected_id: expected })
        }
        let mut pending = PENDING_TX_LIST.load(deps.storage)?;
        let len = match pending.len() {
            0 => 0,
            _ => expected-pending[0],
        };
        if len>MAX_PENDING_LEN{
            return Err(ContractError::UpperBound { max_length: MAX_PENDING_LEN })
        }

        // execution
        let mut old_values = MF_MAP.load(deps.storage, tx.tx_id-1)?;
        let new_values = old_values
        .iter()
        .map(|&item|{
            match item{
                None => None,
                Some(old_value) => {
                    match tx.operation {
                        Operation::DebitBalance { amount } => {
                            if old_value>=amount{
                                Some(old_value-amount)
                            } else {
                                None
                            }
                        },
                        Operation::CreditBalance { amount } => {
                            Some(old_value+amount)
                        },
                    }
                }
            }
        })
        .collect::<Vec<Option<i64>>>();

        // check if we can give instant voting
        let status = check_execution_stautus(&new_values);

        // post execution update
        old_values.extend(new_values);
        MF_MAP.save(deps.storage, tx.tx_id, &old_values)?;
        MF_VOTE_MAP.save(deps.storage, tx.tx_id, match status {
            ExecutionStatus::Success | ExecutionStatus::Failure => &true,
            ExecutionStatus::Uncertainty => &false,
        })?;
        EXPECTED_TX_ID.save(deps.storage, &(expected+1))?;
        pending.push(expected);
        PENDING_TX_LIST.save(deps.storage, &pending)?;

        // response
        let reps = match status {
            ExecutionStatus::Success | ExecutionStatus::Failure => {
                let msg = give_vote(tx.tx_id, CHAIN_ID.load(deps.storage)?, status, MY_CHANNEL.load(deps.storage)?.channel_id, env)?;
                Response::new()
                .add_message(msg)
                .add_attribute("voted", "true")
            },
            ExecutionStatus::Uncertainty => {
                Response::new()
                .add_attribute("voted", "false")
            },
        };
        Ok(reps
            .add_attribute("executed_tx", tx.tx_id.to_string()))
    }

    pub fn finalize_tx(
        deps: &mut DepsMut,
        env: &Env,
        instruction: &Instruction,
    ) -> Result<(Vec<(String, String)>, Vec<IbcMsg>), ContractError> {
        // pre-finalization check
        let mut pending = PENDING_TX_LIST.load(deps.storage)?;
        let expected = EXPECTED_TX_ID.load(deps.storage)?;
        let pos = match pending.iter().position(|&x| {x==instruction.tx_id}) {
            Some(i) => i,
            None => return Err(ContractError::MismatchedFinalizationTxId { sent_id: instruction.tx_id, expected_id: pending }) 
        };

        // finalization
        let n = match instruction.commitment {
            true => 1,
            false => 0,
        };
        let chain_id = CHAIN_ID.load(deps.storage)?;
        let channel_id = MY_CHANNEL.load(deps.storage)?.channel_id;
        let mut msgs: Vec<IbcMsg> = Vec::new();
        let mut attrs: Vec<(String, String)> = vec![("finalized_tx".to_string(), instruction.tx_id.to_string()), ("committed".to_string(), instruction.commitment.to_string())];
        let mut shift = pos;
        for i in instruction.tx_id..expected {
            // update mfs
            let updated_mf = MF_MAP.load(deps.storage, i)?
            .iter()
            .enumerate()
            .filter(|(j,_)|{(j >> shift) & 1 == n})
            .map(|(_, &v)| v)
            .collect::<Vec<Option<i64>>>();
            MF_MAP.save(deps.storage, i, &updated_mf)?;
            shift += 1;

            // vote check
            let voted = MF_VOTE_MAP.load(deps.storage, i)?;
            if !voted {
                let status = check_execution_stautus(&updated_mf);
                match status {
                    ExecutionStatus::Success | ExecutionStatus::Failure => {
                        let msg = give_vote(i, chain_id, status, channel_id.clone(), env)?;
                        msgs.push(msg);
                        attrs.push((format!("newly_voted_tx_{}", i), match status {
                            ExecutionStatus::Success=> "success".to_string(),
                            ExecutionStatus::Failure=> "failure".to_string(),
                            _=>unreachable!(),
                        }));
                        MF_VOTE_MAP.save(deps.storage, i, &true)?;
                    },
                    ExecutionStatus::Uncertainty => {},
                }
            }
        }

        // post-finalization update
        pending.remove(pos);
        PENDING_TX_LIST.save(deps.storage, &pending)?;

        // resp
        Ok((attrs, msgs))
    }
}