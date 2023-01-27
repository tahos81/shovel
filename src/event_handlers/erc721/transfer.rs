use crate::{
    common::types::CairoUint256,
    db::{
        collection::{ContractMetadataCollectionInterface, Erc721CollectionInterface},
        document::{ContractMetadata, Erc721},
    },
    event_handlers::context::Event,
    rpc::metadata::{contract, token},
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
    let sender = event.data[0];
    let recipient = event.data[1];
    let token_id = CairoUint256::new(event.data[2], event.data[3]);

    // Create the event context
    let event_context = Event::new(event, rpc, db);

    let contract_metadata_collection =
        event_context.db.collection::<ContractMetadata>("contract_metadata");
    let erc721_collection = event_context.db.collection::<Erc721>("erc721_tokens");

    if sender == FieldElement::ZERO {
        handle_mint(
            recipient,
            token_id,
            &erc721_collection,
            &contract_metadata_collection,
            event_context,
        )
        .await
    } else if recipient == FieldElement::ZERO {
        handle_burn(sender, token_id, &erc721_collection, event_context).await
    } else {
        handle_transfer(sender, recipient, token_id, &erc721_collection, event_context).await
    }
}

async fn handle_mint(
    recipient: FieldElement,
    token_id: CairoUint256,
    erc721_collection: &Collection<Erc721>,
    contract_metadata_collection: &Collection<ContractMetadata>,
    event_context: Event<'_, '_>,
) -> Result<()> {
    let contract_address = event_context.contract_address();
    let block_id = event_context.block_id();

    let token_uri =
        token::get_token_uri(contract_address, &block_id, event_context.rpc, token_id).await;
    let metadata = token::get_token_metadata(&token_uri).await?;
    let erc721_token = Erc721::new(contract_address, token_id, recipient, token_uri, metadata);

    erc721_collection.insert_erc721(erc721_token).await?;

    let metadata_exists =
        contract_metadata_collection.contract_metadata_exists(contract_address).await?;

    if !metadata_exists {
        let name = contract::get_name(contract_address, &block_id, event_context.rpc).await;
        let symbol = contract::get_symbol(contract_address, &block_id, event_context.rpc).await;
        let contract_metadata = ContractMetadata::new(contract_address, name, symbol);
        contract_metadata_collection.insert_contract_metadata(contract_metadata).await?;
    }

    Ok(())
}

async fn handle_burn(
    sender: FieldElement,
    token_id: CairoUint256,
    erc721_collection: &Collection<Erc721>,
    context: Event<'_, '_>,
) -> Result<()> {
    handle_transfer(sender, FieldElement::ZERO, token_id, erc721_collection, context).await
}

async fn handle_transfer(
    sender: FieldElement,
    recipient: FieldElement,
    token_id: CairoUint256,
    erc721_collection: &Collection<Erc721>,
    context: Event<'_, '_>,
) -> Result<()> {
    let contract_address = context.contract_address();
    let block_number = context.event.block_number;

    erc721_collection
        .update_erc721_owner(contract_address, token_id, sender, recipient, block_number)
        .await
}
