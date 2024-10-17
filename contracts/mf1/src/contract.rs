use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult
};
use crate::error::ContractError;
use crate::msg::{InstantiateMsg, QueryMsg, ExecuteMsg};
use crate::state::{CHAIN_ID, FUTURE_MAP, PENDING_TX_LIST, EXPECTED_TX_ID};

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    CHAIN_ID.save(deps.storage, &msg.chain_id)?;
    PENDING_TX_LIST.save(deps.storage, &Vec::new())?;
    EXPECTED_TX_ID.save(deps.storage, &1)?;
    FUTURE_MAP.save(deps.storage, 0b0, &Some(msg.original_value))?;
    Ok(Response::new()
    .add_attribute("method", "instantiate")
    .add_attribute("initiated_chain", msg.chain_id.to_string())
    .add_attribute("original_value", msg.original_value.to_string()))
}

/* QUERY */
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg {
        AllFutures{} => to_json_binary(&query::all_futures(deps)?),
        ListInfo{} => to_json_binary(&query::list_info(deps)?),
    }
}

mod query {
    use crate::msg::{AllFuturesResp, ListInfoResp};
    use crate::utils;

    use super::*;

    pub fn all_futures(deps: Deps) -> StdResult<AllFuturesResp> {
        let futures = FUTURE_MAP
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| {
            let (k, v) = item?;
            Ok((format!("{:b}b0", k).chars().rev().collect(), match v {
                Some(val) => val.to_string(),
                None => "Failure".to_string(),
            }))
        })
        .collect::<StdResult<Vec<(String, String)>>>()?;

        Ok(AllFuturesResp{ futures })
    }

    pub fn list_info(deps: Deps) -> StdResult<ListInfoResp> {
        let pending_txs = PENDING_TX_LIST.load(deps.storage)?;
        let expected_tx = EXPECTED_TX_ID.load(deps.storage)?;

        Ok(ListInfoResp { pending_txs, expected_tx})
    }
}

/* EXECUTION */
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    use ExecuteMsg::*;

    match msg {
        ExecuteTx{ fcross_tx } => exec::execute_tx(deps, &fcross_tx),
        FinalizeTx{ tx_info } => exec::finalize_tx(deps, &tx_info),
    }
}

mod exec {
    use crate::{error::ContractError, msg::{FcrossTx, TxInfo}, state::MAX_PENDING_LEN, utils};

    use super::*;
    use crate::msg::Operation;

    pub fn execute_tx(
        deps: DepsMut,
        tx: &FcrossTx,
    ) -> Result<Response, ContractError> {
        // check & update pending txs
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
        EXPECTED_TX_ID.save(deps.storage, &(expected+1))?;
        pending.push(expected);
        PENDING_TX_LIST.save(deps.storage, &pending)?;
        

        // get all kv pairs & calculate
        let new_kvpairs= FUTURE_MAP
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| {
            let (k, v) = item?;
            let new_value= match v {
                Some(old_value) => {
                    match tx.operation {
                        Operation::DebitBalance { amount } => {
                            if old_value>=amount {
                                Some(old_value-amount)
                            }
                            else {
                                None
                            }
                        },
                        Operation::CreditBalance { amount } => {Some(old_value+amount)},
                    }
                },
                None => None,
            };
            // note << oepration overflow uncontrolled
            Ok((k+(1u32<<len), new_value))
        })
        .collect::<StdResult<Vec<(u32, Option<i64>)>>>()?;

        //insert new pairs
        new_kvpairs
        .iter()
        .try_for_each(|(k, v)| {
            FUTURE_MAP.save(deps.storage, *k, v)
        })?;

        // let attrs = new_kvpairs.iter().map(|(k, v)| {
        //     (k.to_string(), v.to_string())
        // });

        let resp: Response<_> = Response::new()
        .add_attribute("action", "execute")
        .add_attribute("inserted_keys", utils::keys_format(&new_kvpairs
            .iter()
            .map(|&(k, _)| {k}).
            collect::<Vec<u32>>()));
        
        Ok(resp)
    }

    pub fn finalize_tx(
        deps: DepsMut,
        tx_info: &TxInfo,
    ) -> Result<Response, ContractError> {
        // check & update pending txs
        let mut pending = PENDING_TX_LIST.load(deps.storage)?;
        let index = match pending.iter().position(|&x| {x==tx_info.tx_id}) {
            Some(i) => i,
            None => return Err(ContractError::MismatchedFinalizationTxId { sent_id: tx_info.tx_id, expected_id: pending }) 
        };
        let t = pending[index]-pending[0];
        // update
        pending.remove(index);
        PENDING_TX_LIST.save(deps.storage, &pending)?;

        // get all kv pairs to be deleted
        let removed_keys = FUTURE_MAP
        .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|item|{
            match item {
                Ok(k) => {
                    // k & (1<<t))==0 means key has 0 on its Xth bit
                    match ((k & (1<<t))==0)==tx_info.committed {
                        true => Some(item),
                        false => None,
                    }
                },
                Err(e) => Some(Err(e)),
            }
        })
        .collect::<StdResult<Vec<u32>>>()?;

        // remove
        removed_keys
        .iter()
        .for_each(|item|{
            FUTURE_MAP.remove(deps.storage, *item)
        });

        let resp = Response::new()
        .add_attribute("action", "finalize")
        .add_attribute("removed_keys", utils::keys_format(&removed_keys));

        //update the rest keys if nessasary
        if index == 0 {
            // calculate right shift len
            let t2 = if pending.len()==0 {
                let expected = EXPECTED_TX_ID.load(deps.storage)?;
                expected-tx_info.tx_id
            } else {pending[0]-tx_info.tx_id};

            // generate new kv pairs
            let new_kvs = FUTURE_MAP
            .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .map(|item| item.map(|(k, v)| {
                (k>>t2,v)
            }))
            .collect::<StdResult<Vec<(u32, Option<_>)>>>()?;
            
            // delete existing kv pairs
            FUTURE_MAP
            .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .collect::<StdResult<Vec<u32>>>()?
            .iter()
            .for_each(|k| {FUTURE_MAP.remove(deps.storage, *k);});

            // insert new keys
            new_kvs
            .iter()
            .try_for_each(|(k, v)|{
                FUTURE_MAP.save(deps.storage, *k, v)
            })?;

            // update resp
            return Ok(resp.add_attribute("shrink_length", t2.to_string()));
        }

        Ok(resp)
    }
}