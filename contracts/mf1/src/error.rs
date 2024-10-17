use cosmwasm_std::{Addr, StdError};
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),

    #[error("expect execution tx id {expected_id} but got {sent_id}")]
    MismatchedExecutionTxId {
        sent_id: u32,
        expected_id: u32,
    },
    #[error("expect finalization tx ids {:?} but got {sent_id}", expected_id)]
    MismatchedFinalizationTxId{
        sent_id: u32,
        expected_id: Vec<u32>,
    },

    
    #[error("{sender} is not contract admin")]
    Unauthorized { sender: Addr },
    #[error("Payment error: {0}")]
    PaymentError(#[from] PaymentError),
}