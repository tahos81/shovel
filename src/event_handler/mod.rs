use crate::db::document::ERC721;
use crate::db::NftExt;
use crate::rpc::starknet_constants::*;
use crate::rpc::{self, get_token_uri};
use mongodb::Database;
use starknet::providers::jsonrpc::models::{BlockId, EmittedEvent};
use starknet::providers::jsonrpc::{HttpTransport, JsonRpcClient};

pub async fn handle_transfer_events(
    transfer_events: Vec<EmittedEvent>,
    rpc: &JsonRpcClient<HttpTransport>,
    db: &Database,
) {
    for transfer_event in transfer_events {
        if transfer_event.keys.contains(&TRANSFER_EVENT_KEY) {
            //possible ERC721
            let contract_address = transfer_event.from_address;
            let block_id = BlockId::Number(transfer_event.block_number);
            if rpc::is_erc721(contract_address, block_id, &rpc).await {
                handle_erc721_event(transfer_event, rpc, db).await;
            }
        } else {
            //definitely ERC1155
            handle_erc1155_event(transfer_event);
        }
    }
}

async fn handle_erc721_event(
    erc721_event: EmittedEvent,
    rpc: &JsonRpcClient<HttpTransport>,
    db: &Database,
) {
    if erc721_event.data[0] == ZERO_ADDRESS {
        handle_erc721_mint(erc721_event, rpc, db).await;
    } else if erc721_event.data[1] == ZERO_ADDRESS {
        handle_erc721_burn(erc721_event);
    } else {
        handle_erc721_transfer(erc721_event);
    }
}

async fn handle_erc721_mint(
    erc721_event: EmittedEvent,
    rpc: &JsonRpcClient<HttpTransport>,
    db: &Database,
) {
    let owner = erc721_event.data[1];
    let token_id = erc721_event.data[2];
    let contract_address = erc721_event.from_address;
    let block_id = BlockId::Number(erc721_event.block_number);
    let token_uri = get_token_uri(contract_address, block_id, rpc, token_id).await;
    let new_erc721 = ERC721::new(contract_address, token_id, owner, token_uri);
    db.insert_erc721(new_erc721).await;
}

fn handle_erc721_burn(erc721_event: EmittedEvent) {}

fn handle_erc721_transfer(erc721_event: EmittedEvent) {}

fn handle_erc1155_event(erc1155_event: EmittedEvent) {}
