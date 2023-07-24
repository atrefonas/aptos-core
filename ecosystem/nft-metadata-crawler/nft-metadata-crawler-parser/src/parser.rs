// Copyright © Aptos Foundation

use crate::{
    models::nft_metadata_crawler_uris::NFTMetadataCrawlerURIs, utils::pubsub_entry::PubsubEntry,
};
use diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection,
};

/// Stuct that represents a parser for a single entry from queue
#[allow(dead_code)] // Will remove when functions are implemented
pub struct Parser {
    entry: PubsubEntry,
    model: NFTMetadataCrawlerURIs,
    bucket: String,
    token: String,
    conn: PooledConnection<ConnectionManager<PgConnection>>,
    cdn_prefix: String,
}

impl Parser {
    pub fn new(
        entry: PubsubEntry,
        bucket: String,
        token: String,
        conn: PooledConnection<ConnectionManager<PgConnection>>,
        cdn_prefix: String,
    ) -> Self {
        Self {
            model: NFTMetadataCrawlerURIs::new(entry.token_uri.clone()),
            entry,
            bucket,
            token,
            conn,
            cdn_prefix,
        }
    }

    /// Main parsing flow
    pub async fn parse(&mut self) -> anyhow::Result<()> {
        todo!();
    }
}
