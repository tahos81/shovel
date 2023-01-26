use crate::{
    common::cairo_types::CairoUint256,
    db::{
        collection::{ContractMetadataCollectionInterface, Erc1155CollectionInterface},
        document::{ContractMetadata, Erc1155Balance},
    },
    event_handlers::context::EventContext,
    rpc::metadata::contract,
};
use color_eyre::eyre::Result;
use mongodb::{Collection, Database};
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{models::EmittedEvent, HttpTransport, JsonRpcClient},
};

pub async fn run(
    event: &EmittedEvent,
    rpc: &JsonRpcClient<HttpTransport>,
    db: &Database,
) -> Result<()> {
    let _operator = event.data[0];
    let sender = event.data[1];
    let recipient = event.data[2];
    let token_id = CairoUint256::new(event.data[3], event.data[4]);
    let amount = CairoUint256::new(event.data[5], event.data[6]);
    let erc1155_collection = db.collection::<Erc1155Balance>("erc1155_tokens");
    let contract_metadata_collection = db.collection::<ContractMetadata>("contract_metadata");

    // Create the event context
    let context = EventContext::new(event, rpc, db);

    handle_transfer(
        sender,
        recipient,
        token_id,
        amount,
        &erc1155_collection,
        &contract_metadata_collection,
        &context,
    )
    .await
}

pub async fn handle_transfer(
    sender: FieldElement,
    recipient: FieldElement,
    token_id: CairoUint256,
    amount: CairoUint256,
    erc1155_collection: &Collection<Erc1155Balance>,
    contract_metadata_collection: &Collection<ContractMetadata>,
    context: &EventContext<'_, '_>,
) -> Result<()> {
    let contract_address = context.contract_address();
    let block_id = context.block_id();

    // Update from balance
    if sender != FieldElement::ZERO {
        // We know that from balance won't be zero
        let from_balance =
            match erc1155_collection.get_erc1155_balance(contract_address, token_id, sender).await?
            {
                Some(v) => v,
                None => {
                    println!("Impossible state, from balance 0, using amount as default");
                    amount
                }
            };

        let new_balance = from_balance - amount;
        erc1155_collection
            .update_erc1155_balance(contract_address, token_id, sender, new_balance)
            .await?;
    } else {
        // Check if contract metadata exists
        let metadata_exists =
            contract_metadata_collection.contract_metadata_exists(contract_address).await?;

        if !metadata_exists {
            let name = contract::get_name(contract_address, &block_id, context.rpc).await;
            let symbol = contract::get_symbol(contract_address, &block_id, context.rpc).await;
            let contract_metadata = ContractMetadata::new(contract_address, name, symbol);
            contract_metadata_collection.insert_contract_metadata(contract_metadata).await?;
        }
    }

    // Update to balance
    match erc1155_collection.get_erc1155_balance(contract_address, token_id, sender).await? {
        Some(previous_balance) => {
            let new_balance = previous_balance + amount;
            erc1155_collection
                .update_erc1155_balance(contract_address, token_id, recipient, new_balance)
                .await?;
        }
        None => {
            // Do insert
            erc1155_collection
                .insert_erc1155_balance(Erc1155Balance::new(
                    contract_address,
                    token_id,
                    recipient,
                    amount,
                ))
                .await;
        }
    }

    Ok(())
}
