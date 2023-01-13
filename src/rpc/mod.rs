use crate::common::errors::ConfigError;
use crate::common::starknet_constants::*;
use crate::common::traits::AsciiExt;
use eyre::Result;
use reqwest::Url;
use starknet::core::types::FieldElement;
use starknet::macros::selector;
use starknet::providers::jsonrpc::{
    models::{
        BlockId, ContractAbiEntry::Function, EmittedEvent, EventFilter, EventsPage, FunctionCall,
    },
    HttpTransport, JsonRpcClient,
};
use std::env;

use crate::common::cairo_types::CairoUint256;

pub fn setup_rpc() -> Result<JsonRpcClient<HttpTransport>, ConfigError> {
    let rpc_url = env::var("STARKNET_MAINNET_RPC")?;
    let parsed_url = Url::parse(&rpc_url)?;
    Ok(JsonRpcClient::new(HttpTransport::new(parsed_url)))
}

pub async fn get_transfers_between(
    start_block: u64,
    range: u64,
    rpc: &JsonRpcClient<HttpTransport>,
) -> Result<Vec<EmittedEvent>> {
    let keys: Vec<FieldElement> =
        Vec::from([TRANSFER_EVENT_KEY, TRANSFER_SINGLE_EVENT_KEY, TRANSFER_BATCH_EVENT_KEY]);

    let transfer_filter: EventFilter = EventFilter {
        from_block: Some(BlockId::Number(start_block)),
        to_block: Some(BlockId::Number(start_block + range)),
        address: None,
        keys: Some(keys),
    };

    let mut continuation_token: Option<String> = None;
    let chunk_size: u64 = 1024;

    let mut get_events_resp: EventsPage;
    let mut events: Vec<EmittedEvent> = Vec::new();

    loop {
        get_events_resp =
            rpc.get_events(transfer_filter.clone(), continuation_token, chunk_size).await?;

        events.append(&mut get_events_resp.events);
        continuation_token = get_events_resp.continuation_token;

        if continuation_token.is_none() {
            break Ok(events);
        }
    }
}

pub async fn is_erc721(
    address: FieldElement,
    block_id: &BlockId,
    rpc: &JsonRpcClient<HttpTransport>,
) -> bool {
    let abi = rpc.get_class_at(block_id, address).await.unwrap().abi.unwrap();

    for abi_entry in abi {
        if let Function(function_abi_entry) = abi_entry {
            if function_abi_entry.name == "ownerOf" || function_abi_entry.name == "owner_of" {
                return true;
            }
        }
    }

    false
}

pub async fn get_name(
    address: FieldElement,
    block_id: &BlockId,
    rpc: &JsonRpcClient<HttpTransport>,
) -> String {
    let request = FunctionCall {
        contract_address: address,
        entry_point_selector: NAME_SELECTOR,
        calldata: vec![],
    };

    let result = rpc.call(request, block_id).await.unwrap_or_default();
    let result_felt = result.get(0).unwrap_or(&ZERO_FELT);

    result_felt.to_ascii()
}

pub async fn get_symbol(
    address: FieldElement,
    block_id: &BlockId,
    rpc: &JsonRpcClient<HttpTransport>,
) -> String {
    let request = FunctionCall {
        contract_address: address,
        entry_point_selector: SYMBOL_SELECTOR,
        calldata: vec![],
    };

    let result = rpc.call(request, block_id).await.unwrap_or_default();
    let result_felt = result.get(0).unwrap_or(&ZERO_FELT);

    result_felt.to_ascii()
}

/// Gets the token URI for a given token ID
pub async fn _get_token_uri(
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
            for &v in temp.iter() {
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
