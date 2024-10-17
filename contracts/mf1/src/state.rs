use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const CHAIN_ID: Item<u16> = Item::new("chain_id");
// mainly use PENDING_TX_LIST, assisted with EXPECTED_TX_ID
pub const PENDING_TX_LIST: Item<Vec<u32>> = Item::new("pending_tx_list");
pub const EXPECTED_TX_ID: Item<u32> = Item::new("expected_tx_id");

pub const FUTURE_MAP: Map<u32, Option<i64>> = Map::new("future_map");

pub const MAX_PENDING_LEN: u32 = 12;