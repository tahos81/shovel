mod starknet_constants;

use starknet::core::types::FieldElement;
use starknet::providers::jsonrpc::models::ContractAbiEntry::Function;
use starknet::providers::jsonrpc::models::{EmittedEvent, EventsPage, FunctionCall};
use starknet::providers::jsonrpc::{
    models::BlockId, models::EventFilter, HttpTransport, JsonRpcClient,
};

use crate::db::NftExt;

use crate::db;
use crate::db::document::{Contract, Erc721ID, ERC721};
use starknet_constants::*;

use dotenv::dotenv;
use reqwest::Url;
use std::{env, str};

trait AsciiExt {
    fn to_ascii(&self) -> String;
}

impl AsciiExt for FieldElement {
    fn to_ascii(&self) -> String {
        str::from_utf8(&self.to_bytes_be())
            .unwrap()
            .trim_start_matches("\0")
            .to_string()
    }
}

pub async fn get_transfers() {
    dotenv().ok();

    let rpc = setup_rpc().await;
    let database = db::connect().await;

    let starting_block = 14791;
    let block_range = 50;

    let keys: Vec<FieldElement> = Vec::from([
        TRANSFER_EVENT_KEY,
        TRANSFER_SINGLE_EVENT_KEY,
        TRANSFER_BATCH_EVENT_KEY,
    ]);

    let transfer_filter: EventFilter = EventFilter {
        from_block: Some(BlockId::Number(starting_block)),
        to_block: Some(BlockId::Number(starting_block + block_range)),
        address: None,
        keys: Some(keys),
    };

    let mut continuation_token: Option<String> = None;
    let chunk_size: u64 = 1024;

    let mut events: EventsPage;

    loop {
        events = rpc
            .get_events(transfer_filter.clone(), continuation_token, chunk_size)
            .await
            .unwrap();

        for event in events.events {
            if event.keys.contains(&TRANSFER_EVENT_KEY) {
                //possible ERC721
            } else {
                //definitely ERC1155
            }
        }

        continuation_token = events.continuation_token;

        if continuation_token.is_none() {
            break;
        }
    }
}

async fn setup_rpc() -> JsonRpcClient<HttpTransport> {
    let rpc_url = env::var("STARKNET_MAINNET_RPC").expect("configure your .env file");
    JsonRpcClient::new(HttpTransport::new(Url::parse(&rpc_url).unwrap()))
}

async fn is_erc721(
    address: FieldElement,
    block_id: BlockId,
    rpc: &JsonRpcClient<HttpTransport>,
) -> bool {
    let abi = rpc
        .get_class_at(&block_id, address)
        .await
        .unwrap()
        .abi
        .unwrap();

    for abi_entry in abi {
        if let Function(function_abi_entry) = abi_entry {
            if function_abi_entry.name == "ownerOf" || function_abi_entry.name == "owner_of" {
                return true;
            }
        }
    }

    false
}

async fn get_name(
    address: FieldElement,
    block_id: BlockId,
    rpc: &JsonRpcClient<HttpTransport>,
) -> String {
    let request = FunctionCall {
        contract_address: address,
        entry_point_selector: NAME_SELECTOR,
        calldata: vec![],
    };

    let result = rpc.call(request, &block_id).await.unwrap();
    let result_felt = result.get(0).unwrap();

    result_felt.to_ascii()
}

fn handle_erc721_event(event: EmittedEvent) {}
