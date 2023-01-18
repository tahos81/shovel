use std::{env, fmt::Display};

use color_eyre::eyre::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use starknet::{
    core::types::FieldElement,
    macros::{felt, selector},
    providers::jsonrpc::{
        models::{BlockId, FunctionCall},
        HttpTransport, JsonRpcClient,
    },
};

use crate::common::{cairo_types::CairoUint256, starknet_constants::ZERO_FELT, traits::AsciiExt};

#[derive(Debug, Deserialize, Serialize)]
struct Metadata {
    name: String,
    description: String,
    image: String,
    //attributes: Vec<String>,
}

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
        entry_point_selector: selector!("tokenURI"),
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

pub async fn get_starkrock_metadatas(rpc: &JsonRpcClient<HttpTransport>) -> Result<()> {
    let username = env::var("IPFS_USERNAME").unwrap();
    let password = env::var("IPFS_PASSWORD").unwrap();

    let starkrock_address =
        felt!("0x012f8e318fe04a1fe8bffe005ea4bbd19cb77a656b4f42682aab8a0ed20702f0");
    let block_id = BlockId::Number(18000);
    let token_id = CairoUint256::new(felt!("2"), felt!("0"));
    let token_uri = get_token_uri(starkrock_address, &block_id, rpc, token_id).await;
    let formatted_uri = token_uri.replace("ipfs://", "");

    let base_url = "https://ipfs.infura.io:5001/api/v0/cat?arg=".to_string();

    let url = base_url + &formatted_uri;
    let client = Client::new();
    let req = client.post(url).basic_auth(username, Some(password));
    let resp = req.send().await.unwrap();
    let metadata: Metadata = resp.json().await?;
    dbg!(metadata);
    Ok(())
}
