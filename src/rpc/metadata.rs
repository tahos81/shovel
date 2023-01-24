use std::env;

use crate::common::{
    cairo_types::CairoUint256,
    starknet_constants::{TOKEN_URI_SELECTOR, ZERO_FELT},
    traits::AsciiExt,
};
use crate::db::document::{MetadataType, TokenMetadata};
use color_eyre::eyre::Result;
use reqwest::Client;
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{
        models::{BlockId, FunctionCall},
        HttpTransport, JsonRpcClient,
    },
};

/// Gets the token URI for a given token ID
pub async fn get_token_uri(
    address: FieldElement,
    block_id: &BlockId,
    rpc: &JsonRpcClient<HttpTransport>,
    token_id: CairoUint256,
) -> String {
    // token_uri(uint256) | tokenURI(uint256)
    // uint256 -> [ felt, felt ]
    let request = FunctionCall {
        contract_address: address,
        entry_point_selector: TOKEN_URI_SELECTOR,
        calldata: vec![token_id.low, token_id.high],
    };

    let token_uri_response = match rpc.call(request, block_id).await {
        Ok(felt_array) => felt_array,
        Err(e) => {
            dbg!(e);
            return String::new();
        }
    };

    // If tokenURI function is EIP721Metadata compliant, it should return one felt
    // Otherwise we also consider the case where contracts returns a felt array
    let is_felt_array = token_uri_response.len() > 1;

    if is_felt_array {
        // Create a vector of bytes from the felt array, and for each felt in the array, filter out
        // the 0's and append to the vector
        let mut chars: Vec<u8> = vec![];
        for felt in token_uri_response.iter().skip(1) {
            let temp = felt.to_bytes_be();
            for &v in &temp {
                if v != 0 {
                    chars.push(v);
                }
            }
        }

        // Convert the array to UTF8 string
        String::from_utf8(chars).unwrap_or_default()
    } else {
        // Convert the array to ASCII
        token_uri_response.get(0).unwrap_or(&ZERO_FELT).to_ascii()
    }
}

/// Gets token metadata for a given uri
pub async fn get_token_metadata(uri: String) -> Result<TokenMetadata> {
    let client = Client::new();

    let metadata_type = get_metadata_type(uri);

    match metadata_type {
        MetadataType::Ipfs(uri) => Ok(handle_ipfs_metadata(uri, &client).await?),
        MetadataType::Http(uri) => Ok(handle_http_metadata(uri, &client).await?),
        MetadataType::OnChain(uri) => Ok(handle_onchain_metadata(uri).await?),
    }
}

fn get_metadata_type(uri: String) -> MetadataType {
    if uri.starts_with("ipfs://") {
        MetadataType::Ipfs(uri)
    } else if uri.starts_with("http://") || uri.starts_with("https://") {
        MetadataType::Http(uri)
    } else {
        MetadataType::OnChain(uri)
    }
}

async fn handle_ipfs_metadata(uri: String, client: &Client) -> Result<TokenMetadata> {
    let username = env::var("IPFS_USERNAME").unwrap();
    let password = env::var("IPFS_PASSWORD").unwrap();

    let formatted_uri = uri.trim_start_matches("ipfs://");

    let base_url = "https://ipfs.infura.io:5001/api/v0/cat?arg=".to_string();
    let url = base_url + formatted_uri;

    let req = client.post(url).basic_auth(&username, Some(&password));
    let resp = req.send().await?;
    let metadata: TokenMetadata = resp.json().await?;
    Ok(metadata)
}

async fn handle_http_metadata(uri: String, client: &Client) -> Result<TokenMetadata> {
    let resp = client.get(uri).send().await?;
    let metadata: TokenMetadata = resp.json().await?;
    Ok(metadata)
}

async fn handle_onchain_metadata(uri: String) -> Result<TokenMetadata> {
    let metadata: TokenMetadata = serde_json::from_str(&uri)?;
    Ok(metadata)
}
