// Copyright © Aptos Foundation

use std::io::Cursor;
use std::time::Duration;

use anyhow::Context;
use backoff::future::retry;
use backoff::ExponentialBackoff;
use futures::FutureExt;
use image::imageops::{resize, FilterType};
use image::{DynamicImage, ImageBuffer, ImageFormat, ImageOutputFormat};

use tracing::error;

use crate::get_uri_metadata;

const MAX_FILE_SIZE_BYTES: u32 = 5000000;
const MAX_RETRY_TIME_SECONDS: u64 = 15;

pub struct ImageOptimizer;

impl ImageOptimizer {
    /// Resizes and optimizes image from input URI.
    /// Returns new image as a byte array and its format.
    pub async fn optimize(uri: Option<String>) -> Option<(Vec<u8>, ImageFormat)> {
        match uri {
            Some(uri) => match Self::optimize_image(uri).await {
                Ok((img_bytes, format)) => Some((img_bytes, format)),
                Err(e) => {
                    error!("Error optimizing image: {}", e);
                    None
                },
            },
            None => None,
        }
    }

    /// Resizes and optimizes image from input URI
    async fn optimize_image(img_uri: String) -> anyhow::Result<(Vec<u8>, ImageFormat)> {
        let (_, size) = get_uri_metadata(img_uri.clone()).await?;
        if size > MAX_FILE_SIZE_BYTES {
            return Err(anyhow::anyhow!("File too large, skipping"));
        }

        let op = || {
            async {
                let response = reqwest::get(&img_uri)
                    .await
                    .context("Failed to get image")?;

                let img_bytes = response
                    .bytes()
                    .await
                    .context("Failed to load image bytes")?;

                let format =
                    image::guess_format(&img_bytes).context("Failed to guess image format")?;

                match format {
                    ImageFormat::Gif | ImageFormat::Avif => Ok((img_bytes.to_vec(), format)),
                    _ => {
                        let img = image::load_from_memory(&img_bytes)
                            .context("Failed to load image from memory")?;
                        let resized_image = resize(&img.to_rgb8(), 400, 400, FilterType::Gaussian);
                        Ok((Self::to_bytes(resized_image)?, format))
                    },
                }
            }
            .boxed()
        };

        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(MAX_RETRY_TIME_SECONDS)),
            ..Default::default()
        };

        match retry(backoff, op).await {
            Ok(result) => Ok(result),
            Err(e) => Err(e),
        }
    }

    /// Converts image to JPEG bytes vector
    fn to_bytes(image_buffer: ImageBuffer<image::Rgb<u8>, Vec<u8>>) -> anyhow::Result<Vec<u8>> {
        let dynamic_image = DynamicImage::ImageRgb8(image_buffer);
        let mut byte_store = Cursor::new(Vec::new());
        match dynamic_image.write_to(&mut byte_store, ImageOutputFormat::Jpeg(50)) {
            Ok(_) => Ok(byte_store.into_inner()),
            Err(_) => Err(anyhow::anyhow!("Error converting image to bytes")),
        }
    }
}
