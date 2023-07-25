// Copyright © Aptos Foundation

use crate::{
    models::{
        nft_metadata_crawler_uris::NFTMetadataCrawlerURIs,
        nft_metadata_crawler_uris_query::NFTMetadataCrawlerURIsQuery,
    },
    utils::{
        image_optimizer::ImageOptimizer, json_parser::JSONParser, pubsub_entry::PubsubEntry,
        uri_parser::URIParser,
    },
};
use diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection,
};
use nft_metadata_crawler_utils::gcs::{write_image_to_gcs, write_json_to_gcs};
use tracing::{error, info};

/// Stuct that represents a parser for a single entry from queue
#[allow(dead_code)]
pub struct ParserEntry {
    entry: PubsubEntry,
    model: NFTMetadataCrawlerURIs,
    bucket: String,
    token: String,
    conn: PooledConnection<ConnectionManager<PgConnection>>,
    cdn_prefix: String,
}

impl ParserEntry {
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
        // Deduplicate token_uri
        // Skip if token_uri already exists and not force
        if self.entry.force
            || NFTMetadataCrawlerURIsQuery::get_by_token_uri(
                self.entry.token_uri.clone(),
                &mut self.conn,
            )?
            .is_none()
        {
            info!(
                last_transaction_version = self.entry.last_transaction_version,
                "Starting JSON parse"
            );

            // Parse token_uri
            info!(
                last_transaction_version = self.entry.last_transaction_version,
                "Parsing token_uri for IPFS URI"
            );
            self.model.set_token_uri(self.entry.token_uri.clone());
            let json_uri = URIParser::parse(self.model.get_token_uri())?;

            // Parse JSON for raw_image_uri and raw_animation_uri
            info!(
                last_transaction_version = self.entry.last_transaction_version,
                "Parsing JSON"
            );
            if let Ok((raw_image_uri, raw_animation_uri, json)) = JSONParser::parse(json_uri).await
            {
                self.model.set_raw_image_uri(raw_image_uri);
                self.model.set_raw_animation_uri(raw_animation_uri);

                // Save parsed JSON to GCS
                info!(
                    last_transaction_version = self.entry.last_transaction_version,
                    "Writing JSON to GCS"
                );
                let cdn_json_uri = write_json_to_gcs(
                    self.token.clone(),
                    self.bucket.clone(),
                    self.entry.token_data_id.clone(),
                    json,
                )
                .await
                .ok();
                self.model.set_cdn_json_uri(cdn_json_uri);

                // Commit model to Postgres
                info!(
                    last_transaction_version = self.entry.last_transaction_version,
                    "Committing JSON parse to Postgres"
                );
                self.commit_to_postgres().await;
            } else {
                // Increment retry count if JSON parsing fails
                error!(
                    last_transaction_version = self.entry.last_transaction_version,
                    "JSON parse failed"
                );
                self.model.increment_json_parser_retry_count()
            }
        } else {
            info!(
                last_transaction_version = self.entry.last_transaction_version,
                "Duplicate token_uri, skipping URI parse"
            );
        }

        // Deduplicate raw_image_uri
        // Proceed with image optimization of force or if raw_image_uri has not been parsed
        if self.entry.force
            || self.model.get_raw_image_uri().map_or(true, |uri_option| {
                NFTMetadataCrawlerURIsQuery::get_by_raw_image_uri(uri_option, &mut self.conn)
                    .map_or(true, |uri| uri.is_none())
            })
        {
            info!(
                last_transaction_version = self.entry.last_transaction_version,
                "Starting image optimization"
            );

            // Parse raw_image_uri, use token_uri if parsing fails
            info!(
                last_transaction_version = self.entry.last_transaction_version,
                "Parsing raw_image_uri for IPFS"
            );
            let default_token_uri = self.model.get_token_uri();
            let raw_image_uri = self
                .model
                .get_raw_image_uri()
                .unwrap_or(default_token_uri.clone());
            let img_uri = URIParser::parse(raw_image_uri).unwrap_or(default_token_uri);

            // Resize and optimize image and animation
            info!(
                last_transaction_version = self.entry.last_transaction_version,
                "Optimizing image"
            );
            let image_option = ImageOptimizer::optimize(img_uri).await.ok();

            // Save resized and optimized image to GCS
            if let Some((image, format)) = image_option {
                info!(
                    last_transaction_version = self.entry.last_transaction_version,
                    "Writing image to GCS"
                );
                let cdn_image_uri = write_image_to_gcs(
                    self.token.clone(),
                    format,
                    self.bucket.clone(),
                    self.entry.token_data_id.clone(),
                    image,
                )
                .await
                .ok();
                self.model.set_cdn_image_uri(cdn_image_uri);
            } else {
                // Increment retry count if image is None
                error!(
                    last_transaction_version = self.entry.last_transaction_version,
                    "Image optimization failed"
                );
                self.model.increment_image_optimizer_retry_count();
            }

            // Commit model to Postgres
            info!(
                last_transaction_version = self.entry.last_transaction_version,
                "Committing image optimization to Postgres"
            );
            self.commit_to_postgres().await;
        } else {
            info!(
                last_transaction_version = self.entry.last_transaction_version,
                "Duplicate raw_image_uri, skipping image optimization"
            );
        }

        // Deduplicate raw_animation_uri
        // Proceed with animation optimization force or if raw_animation_uri has not already been parsed
        if let Some(raw_animation_uri) = self.model.get_raw_animation_uri() {
            if self.entry.force
                || NFTMetadataCrawlerURIsQuery::get_by_raw_animation_uri(
                    raw_animation_uri.clone(),
                    &mut self.conn,
                )
                .map_or(true, |uri| uri.is_none())
            {
                info!(
                    last_transaction_version = self.entry.last_transaction_version,
                    "Starting animation optimization"
                );

                // Parse raw_animation_uri, use None if parsing fails
                info!(
                    last_transaction_version = self.entry.last_transaction_version,
                    "Parsing raw_animation_uri for IPFS"
                );
                let animation_uri =
                    URIParser::parse(raw_animation_uri.clone()).unwrap_or(raw_animation_uri);

                // Resize and optimize animation
                info!(
                    last_transaction_version = self.entry.last_transaction_version,
                    "Optimizing animation"
                );
                let animation_option = ImageOptimizer::optimize(animation_uri).await.ok();

                if let Some((animation, format)) = animation_option {
                    // Save resized and optimized animation to GCS
                    info!(
                        last_transaction_version = self.entry.last_transaction_version,
                        "Writing animation to GCS"
                    );
                    let cdn_animation_uri = write_image_to_gcs(
                        self.token.clone(),
                        format,
                        self.bucket.clone(),
                        self.entry.token_data_id.clone(),
                        animation,
                    )
                    .await
                    .ok();
                    self.model.set_cdn_animation_uri(cdn_animation_uri);
                } else {
                    // Increment retry count if animation is None
                    error!(
                        last_transaction_version = self.entry.last_transaction_version,
                        "Animation optimization failed"
                    );
                    self.model.increment_animation_optimizer_retry_count()
                }

                // Commit model to Postgres
                info!(
                    last_transaction_version = self.entry.last_transaction_version,
                    "Committing animation optimization to Postgres"
                );
                self.commit_to_postgres().await;
            } else {
                info!(
                    last_transaction_version = self.entry.last_transaction_version,
                    "Duplicate raw_animation_uri, skipping animation optimization"
                );
            }
        } else {
            info!(
                last_transaction_version = self.entry.last_transaction_version,
                "raw_animation_uri is None, skipping animation optimization"
            );
        }

        Ok(())
    }

    /// Calls and handles error for upserting to Postgres
    async fn commit_to_postgres(&mut self) {
        todo!();
    }
}
