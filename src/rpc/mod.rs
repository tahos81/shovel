pub mod metadata;

use crate::common::errors::ConfigError;
use crate::common::starknet_constants::*;
use crate::common::traits::AsciiExt;
use color_eyre::eyre::Result;
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

pub fn connect() -> Result<JsonRpcClient<HttpTransport>, ConfigError> {
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

        println!("got {} events", get_events_resp.events.len());
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
) -> Result<bool> {
    let abi = match rpc.get_class_at(block_id, address).await?.abi {
        Some(abi) => abi,
        None => return Ok(false),
    };

    for abi_entry in abi {
        if let Function(function_abi_entry) = abi_entry {
            if function_abi_entry.name == "ownerOf" || function_abi_entry.name == "owner_of" {
                return Ok(true);
            }
        }
    }

    Ok(false)
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
    let result = result.get(0).unwrap_or(&ZERO_FELT);

    result.to_ascii()
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
    let result = result.get(0).unwrap_or(&ZERO_FELT);

    result.to_ascii()
}
