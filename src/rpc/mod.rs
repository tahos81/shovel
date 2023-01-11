pub mod starknet_constants;

use crate::common::traits::AsciiExt;
use reqwest::Url;
use starknet::core::types::FieldElement;
use starknet::providers::jsonrpc::{
    models::{
        BlockId, ContractAbiEntry::Function, EmittedEvent, EventFilter, EventsPage, FunctionCall,
    },
    HttpTransport, JsonRpcClient,
};
use starknet_constants::*;
use std::{env, str};

use crate::common::cairo_types::CairoUint256;

pub fn setup_rpc() -> JsonRpcClient<HttpTransport> {
    let rpc_url = env::var("STARKNET_MAINNET_RPC").expect("configure your .env file");
    JsonRpcClient::new(HttpTransport::new(Url::parse(&rpc_url).unwrap()))
}

pub async fn get_transfers_between(
    start_block: u64,
    range: u64,
    rpc: &JsonRpcClient<HttpTransport>,
) -> Vec<EmittedEvent> {
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

    let mut events_resp: EventsPage;
    let mut events: Vec<EmittedEvent> = Vec::new();

    loop {
        events_resp =
            rpc.get_events(transfer_filter.clone(), continuation_token, chunk_size).await.unwrap();

        events.append(&mut events_resp.events);

        continuation_token = events_resp.continuation_token;

        if continuation_token.is_none() {
            break events;
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

    let result = rpc.call(request, block_id).await.unwrap_or_default();
    let result_felt = result.get(0).unwrap_or(&ZERO_FELT);

    result_felt.to_ascii()
}
