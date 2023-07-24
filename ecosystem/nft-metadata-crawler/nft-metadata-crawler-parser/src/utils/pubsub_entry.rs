// Copyright © Aptos Foundation

/// Struct to hold a single entry from PubSub
#[derive(Clone, Debug)]
pub struct PubsubEntry {
    pub token_data_id: String,
    pub token_uri: String,
    pub last_transaction_version: i32,
    pub last_transaction_timestamp: chrono::NaiveDateTime,
    pub force: bool,
}
