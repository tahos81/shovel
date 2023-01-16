use crate::common::starknet_constants::*;
use crate::db::document::Erc1155Balance;
use crate::db::document::{ContractMetadata, Erc721};
use crate::rpc;
use crate::{
    common::cairo_types::CairoUint256,
    db::collection::{
        ContractMetadataCollectionInterface, Erc1155CollectionInterface, Erc721CollectionInterface,
    },
};
use color_eyre::eyre::Result;
use mongodb::{Collection, Database};
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
) -> Result<()> {
    let mut blacklist: HashSet<FieldElement> = HashSet::new();

    let contract_metadata_collection = db.collection::<ContractMetadata>("contract_metadata");
    let erc721_collection = db.collection::<Erc721>("erc721_tokens");
    let erc1155_collection = db.collection::<Erc1155Balance>("erc1155_token_balances");

    for transfer_event in transfer_events {
        if transfer_event.keys.contains(&TRANSFER_EVENT_KEY) {
            //possible ERC721
            let contract_address = transfer_event.from_address;
            let block_id = BlockId::Number(transfer_event.block_number);
            //probably should hardcode ether address
            if !blacklist.contains(&contract_address)
                && rpc::is_erc721(contract_address, &block_id, rpc).await?
            {
                println!("handling ERC721 event");
                handle_erc721_event(
                    transfer_event,
                    rpc,
                    &erc721_collection,
                    &contract_metadata_collection,
                )
                .await?;
            } else if !blacklist.contains(&contract_address) {
                println!("Blacklisting contract");
                blacklist.insert(contract_address);
            }
        } else if transfer_event.keys.contains(&TRANSFER_SINGLE_EVENT_KEY) {
            println!("handling ERC1155 single event");
            handle_erc1155_transfer_single(transfer_event, &erc1155_collection).await?;
        } else if transfer_event.keys.contains(&TRANSFER_BATCH_EVENT_KEY) {
            println!("handling ERC1155 batch event");
            handle_erc1155_transfer_batch(transfer_event, &erc1155_collection).await?;
        }
    }
    Ok(())
}

async fn handle_erc721_event(
    erc721_event: EmittedEvent,
    rpc: &JsonRpcClient<HttpTransport>,
    erc721_collection: &Collection<Erc721>,
    contract_metadata_collection: &Collection<ContractMetadata>,
) -> Result<()> {
    if erc721_event.data[0] == ZERO_FELT {
        handle_erc721_mint(erc721_event, rpc, erc721_collection, contract_metadata_collection)
            .await?;
        Ok(())
    } else {
        handle_erc721_transfer(erc721_event, erc721_collection).await?;
        Ok(())
    }
}

async fn handle_erc721_mint(
    erc721_event: EmittedEvent,
    rpc: &JsonRpcClient<HttpTransport>,
    erc721_collection: &Collection<Erc721>,
    contract_metadata_collection: &Collection<ContractMetadata>,
) -> Result<()> {
    let owner = erc721_event.data[1];
    let token_id = CairoUint256::new(erc721_event.data[2], erc721_event.data[3]);
    let contract_address = erc721_event.from_address;
    let block_id = BlockId::Number(erc721_event.block_number);

    let new_erc721 = Erc721::new(contract_address, token_id, owner, None);
    erc721_collection.insert_erc721(new_erc721).await?;

    if !contract_metadata_collection.contract_metadata_exists(contract_address).await? {
        let name = rpc::get_name(contract_address, &block_id, rpc).await;
        let symbol = rpc::get_symbol(contract_address, &block_id, rpc).await;
        let new_contract = ContractMetadata::new(contract_address, name, symbol);
        contract_metadata_collection.insert_contract_metadata(new_contract).await?;
    }
    Ok(())
}

async fn handle_erc721_transfer(
    erc721_event: EmittedEvent,
    erc721_collection: &Collection<Erc721>,
) -> Result<()> {
    let old_owner = erc721_event.data[0];
    let new_owner = erc721_event.data[1];
    let token_id = CairoUint256::new(erc721_event.data[2], erc721_event.data[3]);

    let contract_address = erc721_event.from_address;
    let block_number = erc721_event.block_number;
    erc721_collection
        .update_erc721_owner(contract_address, token_id, old_owner, new_owner, block_number)
        .await?;
    Ok(())
}

async fn erc1155_single_transfer(
    from_address: FieldElement,
    to_address: FieldElement,
    token_id: CairoUint256,
    amount: CairoUint256,
    contract_address: FieldElement,
    erc1155_collection: &Collection<Erc1155Balance>,
) -> Result<()> {
    // Update from balance
    if from_address != ZERO_FELT {
        // We know that from balance won't be zero
        let from_balance = match erc1155_collection
            .get_erc1155_balance(contract_address, token_id, from_address)
            .await?
        {
            Some(v) => v,
            None => {
                println!("Impossible state, from balance 0, using amount as default");
                amount
            }
        };

        let new_balance = from_balance - amount;
        erc1155_collection
            .update_erc1155_balance(contract_address, token_id, from_address, new_balance)
            .await?;
    }

    // Update to balance
    match erc1155_collection.get_erc1155_balance(contract_address, token_id, from_address).await? {
        Some(previous_balance) => {
            let new_balance = previous_balance + amount;
            erc1155_collection
                .update_erc1155_balance(contract_address, token_id, to_address, new_balance)
                .await?;
        }
        None => {
            // Do insert
            erc1155_collection
                .insert_erc1155_balance(Erc1155Balance::new(
                    contract_address,
                    token_id,
                    to_address,
                    amount,
                ))
                .await?;
        }
    }
    Ok(())
}

async fn handle_erc1155_transfer_single(
    erc1155_event: EmittedEvent,
    erc1155_collection: &Collection<Erc1155Balance>,
) -> Result<()> {
    let contract_address = erc1155_event.from_address;
    let from_address = erc1155_event.data[1];
    let to_address = erc1155_event.data[2];
    let token_id = CairoUint256::new(erc1155_event.data[3], erc1155_event.data[4]);
    let amount = CairoUint256::new(erc1155_event.data[5], erc1155_event.data[6]);

    erc1155_single_transfer(
        from_address,
        to_address,
        token_id,
        amount,
        contract_address,
        erc1155_collection,
    )
    .await?;
    Ok(())
}

async fn handle_erc1155_transfer_batch(
    erc1155_event: EmittedEvent,
    erc1155_collection: &Collection<Erc1155Balance>,
) -> Result<()> {
    let contract_address = erc1155_event.from_address;
    let from_address = erc1155_event.data[1];
    let to_address = erc1155_event.data[2];

    // Get the length of the token ids array
    let token_length: u32 = erc1155_event.data[3].try_into()?;
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
        erc1155_single_transfer(
            from_address,
            to_address,
            token_id,
            amount,
            contract_address,
            erc1155_collection,
        )
        .await?;
    }
    Ok(())
}
