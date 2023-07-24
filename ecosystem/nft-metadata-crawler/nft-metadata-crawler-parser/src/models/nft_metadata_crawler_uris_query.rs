// Copyright © Aptos Foundation

use crate::schema::nft_metadata_crawler_uris;
use backoff::{retry, ExponentialBackoff};
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const MAX_RETRY_TIME_SECONDS: u64 = 15;

#[derive(Debug, Deserialize, Identifiable, Queryable, Serialize)]
#[diesel(primary_key(token_uri))]
#[diesel(table_name = nft_metadata_crawler_uris)]
pub struct NFTMetadataCrawlerURIsQuery {
    pub token_uri: String,
    pub raw_image_uri: Option<String>,
    pub raw_animation_uri: Option<String>,
    pub cdn_json_uri: Option<String>,
    pub cdn_image_uri: Option<String>,
    pub cdn_animation_uri: Option<String>,
    pub json_parser_retry_count: i32,
    pub image_optimizer_retry_count: i32,
    pub animation_optimizer_retry_count: i32,
    pub inserted_at: chrono::NaiveDateTime,
}

impl NFTMetadataCrawlerURIsQuery {
    pub fn get_by_token_uri(
        token_uri: String,
        conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    ) -> anyhow::Result<Option<Self>> {
        let mut op = || {
            nft_metadata_crawler_uris::table
                .find(token_uri.clone())
                .first::<NFTMetadataCrawlerURIsQuery>(conn)
                .optional()
                .map_err(Into::into)
        };

        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(MAX_RETRY_TIME_SECONDS)),
            ..Default::default()
        };

        match retry(backoff, &mut op) {
            Ok(result) => Ok(result),
            Err(_) => Ok(op()?),
        }
    }

    pub fn get_by_raw_image_uri(
        raw_image_uri: String,
        conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    ) -> anyhow::Result<Option<Self>> {
        let mut op = || {
            nft_metadata_crawler_uris::table
                .filter(nft_metadata_crawler_uris::raw_image_uri.eq(raw_image_uri.clone()))
                .first::<NFTMetadataCrawlerURIsQuery>(conn)
                .optional()
                .map_err(Into::into)
        };

        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(MAX_RETRY_TIME_SECONDS)),
            ..Default::default()
        };

        match retry(backoff, &mut op) {
            Ok(result) => Ok(result),
            Err(_) => Ok(op()?),
        }
    }

    pub fn get_by_raw_animation_uri(
        raw_animation_uri: Option<String>,
        conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    ) -> anyhow::Result<Option<Self>> {
        let mut op = || {
            nft_metadata_crawler_uris::table
                .filter(nft_metadata_crawler_uris::raw_animation_uri.eq(raw_animation_uri.clone()))
                .first::<NFTMetadataCrawlerURIsQuery>(conn)
                .optional()
                .map_err(Into::into)
        };

        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(MAX_RETRY_TIME_SECONDS)),
            ..Default::default()
        };

        match retry(backoff, &mut op) {
            Ok(result) => Ok(result),
            Err(_) => Ok(op()?),
        }
    }
}
