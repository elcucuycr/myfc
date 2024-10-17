use cosmwasm_std::{Addr};
use cosmwasm_schema::{cw_serde, QueryResponses};

/* Initiate */
#[cw_serde]
pub struct InstantiateMsg {
    pub chain_id: u16,
    pub original_value: i64,
}

/* Execute */
#[cw_serde]
pub enum ExecuteMsg {
    ExecuteTx { fcross_tx: FcrossTx },
    FinalizeTx { tx_info: TxInfo },
}

#[cw_serde]
pub struct FcrossTx{
    pub tx_id: u32,
    pub operation: Operation,
}

#[cw_serde]
pub enum Operation {
    CreditBalance { amount: i64 },
    DebitBalance { amount: i64 },
}

#[cw_serde]
pub struct TxInfo{
    pub tx_id: u32,
    pub committed: bool,
}

/* Query */
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GreetResp)]
    Greet {},
    #[returns(AllFuturesResp)]
    AllFutures {},
    #[returns(PendingListLengthResp)]
    PendingListLength {},
}

#[cw_serde]
pub struct GreetResp {
    pub message: String,
}

#[cw_serde]
pub struct PendingListLengthResp {
    pub len: u32,
}

#[cw_serde]
pub struct AllFuturesResp {
    pub futures: Vec<(String, String)>,
}
