use crate::common::starknet_constants::*;
use crate::db::document::ERC1155Balance;
use crate::rpc;
use crate::{
    common::cairo_types::CairoUint256,
    db::{
        document::{Contract, ERC721},
        NftExt,
    },
};
use mongodb::Database;
use starknet::core::types::FieldElement;
use starknet::providers::jsonrpc::{
    models::{BlockId, EmittedEvent},
    HttpTransport, JsonRpcClient,
};

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
            //probably should hardcode ether address
            if !blacklist.contains(&contract_address)
                && rpc::is_erc721(contract_address, &block_id, rpc).await
            {
                handle_erc721_event(transfer_event, rpc, db).await;
            } else {
                blacklist.insert(contract_address);
            }
        } else if transfer_event.keys.contains(&TRANSFER_SINGLE_EVENT_KEY) {
            handle_erc1155_transfer_single(transfer_event, db).await;
        } else if transfer_event.keys.contains(&TRANSFER_BATCH_EVENT_KEY) {
            handle_erc1155_transfer_batch(transfer_event, db).await;
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
    let token_id = CairoUint256::new(erc721_event.data[2], erc721_event.data[3]);
    let contract_address = erc721_event.from_address;
    let block_id = BlockId::Number(erc721_event.block_number);

    let new_erc721 = ERC721::new(contract_address, token_id, owner, None);
    db.insert_erc721(new_erc721).await;

    if !db.contract_exists(contract_address).await {
        let name = rpc::get_name(contract_address, &block_id, rpc).await;
        let symbol = rpc::get_symbol(contract_address, &block_id, rpc).await;
        let new_contract = Contract::new(contract_address, name, symbol);
        db.insert_contract(new_contract).await;
    }
}

async fn handle_erc721_transfer(erc721_event: EmittedEvent, db: &Database) {
    let old_owner = erc721_event.data[0];
    let new_owner = erc721_event.data[1];
    let token_id = CairoUint256::new(erc721_event.data[2], erc721_event.data[3]);

    let contract_address = erc721_event.from_address;
    let block_number = erc721_event.block_number;
    db.update_erc721_owner(contract_address, token_id, old_owner, new_owner, block_number).await;
}

async fn erc1155_single_transfer(
    from_address: FieldElement,
    to_address: FieldElement,
    token_id: CairoUint256,
    amount: CairoUint256,
    contract_address: FieldElement,
    db: &Database,
) {
    // Update from balance
    if from_address != ZERO_FELT {
        // We know that from balance won't be zero
        let from_balance =
            db.get_erc1155_balance(contract_address, token_id, from_address).await.unwrap();

        let new_balance = from_balance - amount;
        db.update_erc1155_balance(contract_address, token_id, from_address, new_balance).await;
    }

    // Update to balance
    match db.get_erc1155_balance(contract_address, token_id, from_address).await {
        Some(previous_balance) => {
            let new_balance = previous_balance + amount;
            db.update_erc1155_balance(contract_address, token_id, to_address, new_balance).await;
        }
        None => {
            // Do insert
            db.insert_erc1155_balance(ERC1155Balance::new(
                contract_address,
                token_id,
                to_address,
                amount,
            ))
            .await;
        }
    }
}

async fn handle_erc1155_transfer_single(erc1155_event: EmittedEvent, db: &Database) {
    let contract_address = erc1155_event.from_address;
    let from_address = erc1155_event.data[1];
    let to_address = erc1155_event.data[2];
    let token_id = CairoUint256::new(erc1155_event.data[3], erc1155_event.data[4]);
    let amount = CairoUint256::new(erc1155_event.data[5], erc1155_event.data[6]);

    erc1155_single_transfer(from_address, to_address, token_id, amount, contract_address, db).await;
}

async fn handle_erc1155_transfer_batch(erc1155_event: EmittedEvent, db: &Database) {
    let contract_address = erc1155_event.from_address;
    let from_address = erc1155_event.data[1];
    let to_address = erc1155_event.data[2];

    // Get the length of the token ids array
    let token_length: u32 = erc1155_event.data[3].try_into().unwrap();
    let token_length = token_length as usize;

    // This is index difference between token id and corresponding amount in the event data array
    let amount_delta = token_length * 2 + 1;

    // Zip token ids and amounts together
    let single_transfers = erc1155_event.data[4..(3 + amount_delta)]
        .chunks(2)
        .map(|chunk| CairoUint256::new(chunk[0], chunk[1]))
        .zip(
            erc1155_event.data[(4 + amount_delta)..]
                .chunks(2)
                .map(|chunk| CairoUint256::new(chunk[0], chunk[1])),
        );

    // For each token_id - amount pair, process a single transfer
    for (token_id, amount) in single_transfers {
        erc1155_single_transfer(from_address, to_address, token_id, amount, contract_address, db)
            .await;
    }
}
