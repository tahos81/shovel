use std::str::FromStr;

use crate::db::document::TokenMetadata;
use aws_sdk_s3::input::PutBucketOwnershipControlsInput;
use color_eyre::eyre::{private::kind::TraitKind, Result};

use super::{s3::AwsS3Storage, svg_to_png::svg_to_png};

const MAX_CONTENT_LENGTH: u32 = 8 * 1024 * 1024 * 50;

pub async fn store_metadata(
    key: &str,
    metadata: TokenMetadata,
    storage: &AwsS3Storage,
) -> Option<String> {
    if let Some(image) = metadata.image {
        let url = match reqwest::Url::from_str(image.as_str()) {
            Ok(v) => v,
            _ => return None,
        };

        let content_length = match get_content_length(url.clone()).await {
            Some(length) => length,
            // HEAD request failed or Content-Length header missing
            _ => return None,
        };

        if content_length > MAX_CONTENT_LENGTH {
            dbg!(format!("Content length exceeds max length {}", content_length));
            return None;
        }

        // If successful request, get response bytes
        let image_request = reqwest::get(url).await.map(|response| response.bytes());

        let image_data = match image_request {
            Ok(response) => response.await.ok(),
            Err(e) => {
                dbg!("Couldn't fetch image", format!("{:?}", e));
                None
            }
        };

        if let Some(image_data) = image_data {
            storage.upload("shovel", key, image_data.to_vec()).await.ok()
        } else {
            None
        }
    } else if let Some(image_data) = metadata.image_data {
        // Image data holds raw SVG data, cache it as PNG
        let svg_bytes = image_data.as_bytes();
        let png_converted = match svg_to_png(svg_bytes) {
            Ok(data) => Some(data),
            Err(e) => {
                dbg!("SVG conversion failed", format!("{:?}", e));
                None
            }
        };

        if let Some(png_data) = png_converted {
            storage.upload("shovel", key, png_data).await.ok()
        } else {
            None
        }
    } else if let Some(animation_url) = metadata.animation_url {
        let url = match reqwest::Url::from_str(animation_url.as_str()) {
            Ok(v) => v,
            _ => return None,
        };

        let content_length = match get_content_length(url.clone()).await {
            Some(length) => length,
            // HEAD request failed or Content-Length header missing
            _ => return None,
        };

        if content_length > MAX_CONTENT_LENGTH {
            dbg!(format!("Content length exceeds max length {}", content_length));
            return None;
        }

        // If successful request, get response bytes
        let animation_request = reqwest::get(url).await.map(|response| response.bytes());

        let animation_data = match animation_request {
            Ok(response) => response.await.ok(),
            Err(e) => {
                dbg!("Couldn't fetch image", format!("{:?}", e));
                None
            }
        };

        if let Some(animation_data) = animation_data {
            storage.upload("shovel", key, animation_data.to_vec()).await.ok()
        } else {
            None
        }
    } else {
        println!("No metadata found");
        None
    }
}

/// Extracts content length from URL
async fn get_content_length(url: reqwest::Url) -> Option<u32> {
    let client = reqwest::Client::new();
    let header_request = client.head(url).send().await;

    if let Ok(head_response) = header_request {
        head_response
            .headers()
            .get("Content-Length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| str::parse::<u32>(v).ok())
    } else {
        // Header request failed, return empty
        None
    }
}
