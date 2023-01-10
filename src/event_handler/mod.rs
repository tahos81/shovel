use crate::db::document::*;
use crate::db::NftExt;
use crate::rpc::starknet_constants::*;
use crate::rpc::{self, get_name, get_symbol, get_token_uri};
use mongodb::Database;
use starknet::core::types::FieldElement;
use starknet::providers::jsonrpc::models::{BlockId, EmittedEvent};
use starknet::providers::jsonrpc::{HttpTransport, JsonRpcClient};

use std::collections::HashSet;

pub async fn handle_transfer_events(
    transfer_events: Vec<EmittedEvent>,
    rpc: &JsonRpcClient<HttpTransport>,
    db: &Database,
) {
    let mut blacklist: HashSet<FieldElement> = HashSet::new();
    for transfer_event in transfer_events {
        if transfer_event.keys.contains(&TRANSFER_EVENT_KEY) {
            //possible ERC721
            let contract_address = transfer_event.from_address;
            let block_id = BlockId::Number(transfer_event.block_number);
            if !blacklist.contains(&contract_address)
                && rpc::is_erc721(contract_address, &block_id, &rpc).await
            {
                handle_erc721_event(transfer_event, rpc, db).await;
            } else {
                blacklist.insert(contract_address);
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
    if erc721_event.data[0] == ZERO_FELT {
        handle_erc721_mint(erc721_event, rpc, db).await;
    } else if erc721_event.data[1] == ZERO_FELT {
        handle_erc721_burn(erc721_event);
    } else {
        handle_erc721_transfer(erc721_event, db).await;
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
    let token_uri = get_token_uri(contract_address, &block_id, rpc, token_id).await;
    let new_erc721 = ERC721::new(contract_address, token_id, owner, token_uri);
    db.insert_erc721(new_erc721).await;
    if !db.contract_exists(contract_address).await {
        let name = get_name(contract_address, &block_id, rpc).await;
        let symbol = get_symbol(contract_address, &block_id, rpc).await;
        let new_contract = Contract::new(contract_address, name, symbol);
        db.insert_contract(new_contract).await;
    }
}

async fn handle_erc721_transfer(erc721_event: EmittedEvent, db: &Database) {
    let old_owner = erc721_event.data[0];
    let new_owner = erc721_event.data[1];
    let token_id = erc721_event.data[2];
    let contract_address = erc721_event.from_address;
    let block_number = erc721_event.block_number;
    db.update_erc721_owner(
        contract_address,
        token_id,
        old_owner,
        new_owner,
        block_number,
    )
    .await;
}

fn handle_erc721_burn(erc721_event: EmittedEvent) {}

fn handle_erc1155_event(erc1155_event: EmittedEvent) {}
