use cosmwasm_std::{Deps, StdResult};

use crate::state::{EXPECTED_TX_ID, PENDING_TX_LIST};

pub fn future_index_to_string(index: u16) -> String {
    format!("{:b}", index)
    .chars()
    .rev()
    .collect()
}

pub fn calculate_pending_list_len(deps: Deps) -> StdResult<u32> {
    let list = PENDING_TX_LIST.load(deps.storage)?;
    if list.len()==0 {
        Ok(0)
    } else {
        Ok(EXPECTED_TX_ID.load(deps.storage)?-list[0])
    }
}

pub fn keys_format(ks: &Vec<u32>) -> String {
    ks
    .iter()
    .map(|&i| format!("{:b}", i).chars().rev().collect())
    .collect::<Vec<String>>()
    .join(",")
}