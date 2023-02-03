use std::str::FromStr;

use crate::db::document::TokenMetadata;

use super::{s3::AwsS3Storage, svg_to_png::svg_to_png};

const MAX_CONTENT_LENGTH: u32 = 8 * 1024 * 1024 * 50;

enum UriType<'a> {
    Ipfs(&'a str),
    Http(&'a str),
    Other(&'a str),
}

fn get_uri_type(uri: &str) -> UriType<'_> {
    if uri.starts_with("ipfs://") {
        UriType::Ipfs(uri)
    } else if uri.starts_with("http://") || uri.starts_with("https://") {
        UriType::Http(uri)
    } else {
        UriType::Other(uri)
    }
}

pub async fn store_metadata(
    key: &str,
    metadata: &TokenMetadata,
    storage: &AwsS3Storage,
) -> Option<String> {
    if let Some(image) = &metadata.image {
        let uri_type = get_uri_type(image);

        let image_data = match uri_type {
            UriType::Http(http_url) => fetch_http_content(http_url).await,
            UriType::Ipfs(ipfs_url) => {
                let ipfs_hash = ipfs_url.trim_start_matches("ipfs://");
                let ipfs_url = format!("https://cloudflare-ipfs.com/ipfs/{}", ipfs_hash);
                fetch_http_content(&ipfs_url).await
            }
            UriType::Other(other_url) => {
                eprintln!("Unknown url {}", other_url);
                None
            }
        };

        if let Some((ctype, data)) = image_data {
            Some(storage.upload("shovel-metadata", key, &ctype, data).await.unwrap())
        } else {
            None
        }
    } else if let Some(image_data) = &metadata.image_data {
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
            storage.upload("shovel-metadata", key, "image/png", png_data).await.ok()
        } else {
            None
        }
    } else if let Some(animation_url) = &metadata.animation_url {
        let uri_type = get_uri_type(animation_url);

        let anim_data = match uri_type {
            UriType::Http(http_url) => fetch_http_content(http_url).await,
            UriType::Ipfs(ipfs_url) => {
                let ipfs_hash = ipfs_url.trim_start_matches("ipfs://");
                let ipfs_url = format!("https://cloudflare-ipfs.com/ipfs/{}", ipfs_hash);
                fetch_http_content(&ipfs_url).await
            }
            UriType::Other(other_url) => {
                eprintln!("Unknown url {}", other_url);
                None
            }
        };

        if let Some((ctype, data)) = anim_data {
            Some(storage.upload("shovel-metadata", key, &ctype, data).await.unwrap())
        } else {
            None
        }
    } else {
        eprintln!("No metadata found");
        None
    }
}

async fn fetch_http_content(url: &str) -> Option<(String, Vec<u8>)> {
    let url = match reqwest::Url::from_str(url) {
        Ok(v) => v,
        _ => {
            eprintln!("Failed to parse url {}", url);
            return None;
        }
    };

    let (content_type, content_length) = match get_content_type_and_length(url.clone()).await {
        Some((t, l)) => (t, l),
        // HEAD request failed or Content-Length header missing
        _ => {
            eprintln!("Content length doesn't exist");
            return None;
        }
    };

    if content_length > MAX_CONTENT_LENGTH {
        dbg!(format!("Content length exceeds max length {}", content_length));
        return None;
    }

    // If successful request, get response bytes
    let url_request = reqwest::get(url).await.map(|response| response.bytes());

    match url_request {
        Ok(response) => response.await.ok().map(|v| (content_type, v.to_vec())),
        Err(e) => {
            dbg!("Couldn't fetch data", format!("{:?}", e));
            None
        }
    }
}

/// Extracts content length from URL
async fn get_content_type_and_length(url: reqwest::Url) -> Option<(String, u32)> {
    let client = reqwest::Client::new();
    let header_request = client.head(url).send().await;

    if let Ok(head_response) = header_request {
        let headers = head_response.headers();

        let content_length = headers
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| str::parse::<u32>(v).ok());

        let content_type =
            headers.get("content-type").and_then(|v| v.to_str().ok()).map(|v| v.to_string());

        content_type.zip(content_length)
    } else {
        eprintln!("Header request failed");
        // Header request failed, return empty
        None
    }
}
