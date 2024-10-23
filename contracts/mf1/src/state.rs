use cosmwasm_schema::cw_serde;
use cw_storage_plus::{Item, Map};

pub const CHAIN_ID: Item<u16> = Item::new("chain_id");

// mainly use PENDING_TX_LIST, assisted with EXPECTED_TX_ID
pub const PENDING_TX_LIST: Item<Vec<u32>> = Item::new("pending_tx_list"); // pending changes when instruction comes
pub const EXPECTED_TX_ID: Item<u32> = Item::new("expected_tx_id");

pub const MF_MAP: Map<u32, Vec<Option<i64>>> = Map::new("mf_maps");
pub const MF_VOTE_MAP: Map<u32, bool> = Map::new("mf_vote_maps"); // only used to record if the mf has voted

pub const MAX_PENDING_LEN: u32 = 12;

// ibc relevant state
pub const MY_CHANNEL: Item<ChannelInfo> = Item::new("my_channel");

#[cw_serde]
pub struct ChannelInfo {
    pub channel_id: String,
    /// whether the channel is completely set up
    pub finalized: bool,
}

pub const MY_LOGS: Item<String> = Item::new("my_logs");