mod event_keys;

use starknet::core::types::FieldElement;
use starknet::providers::jsonrpc::models::ContractAbiEntry::Function;
use starknet::providers::jsonrpc::models::{EmittedEvent, EventsPage};
use starknet::providers::jsonrpc::{
    models::BlockId, models::EventFilter, HttpTransport, JsonRpcClient,
};

use event_keys::*;

use dotenv::dotenv;
use reqwest::Url;
use std::collections::HashSet;
use std::env;

pub async fn get_transfers() -> Vec<EmittedEvent> {
    dotenv().ok();

    let mut ret: Vec<EmittedEvent> = Vec::new();

    //TODO: Replace it with database
    let mut whitelist: HashSet<FieldElement> = HashSet::new();
    let mut blacklist: HashSet<FieldElement> = HashSet::new();

    let rpc_url = env::var("STARKNET_MAINNET_RPC").expect("configure your .env file");
    let rpc = JsonRpcClient::new(HttpTransport::new(Url::parse(&rpc_url).unwrap()));

    let keys: Vec<FieldElement> = Vec::from([
        TRANSFER_EVENT_KEY,
        TRANSFER_SINGLE_EVENT_KEY,
        TRANSFER_BATCH_EVENT_KEY,
    ]);

    let starting_block = 14791;
    let block_range = 0;

    let filter: EventFilter = EventFilter {
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
            //HOW CAN I REFRAIN FROM CLONING FILTER
            .get_events(filter.clone(), continuation_token, chunk_size)
            .await
            .unwrap();

        for event in events.events {
            if event.keys.contains(&TRANSFER_EVENT_KEY) {
                //possible ERC721
                let address = event.from_address;
                if whitelist.contains(&address) {
                    //dbg!(event);
                    continue;
                } else if blacklist.contains(&address) {
                    continue;
                } else {
                    if is_erc721(address, BlockId::Number(event.block_number), &rpc).await {
                        whitelist.insert(address);
                        ret.push(event);
                        //dbg!(event);
                    } else {
                        blacklist.insert(address);
                    }
                }
            } else {
                //definitely ERC1155
                //dbg!(event);
            }
        }

        continuation_token = events.continuation_token;

        if continuation_token.is_none() {
            break;
        }
    }

    ret
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
        match abi_entry {
            Function(function_abi_entry) => {
                if function_abi_entry.name == "ownerOf" || function_abi_entry.name == "owner_of" {
                    return true;
                }
            }
            _ => {}
        }
    }

    false
}
