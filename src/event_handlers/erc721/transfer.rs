use crate::{
    common::{starknet_constants::ZERO_FELT, types::CairoUint256},
    db::{
        collection::{ContractMetadataCollectionInterface, Erc721CollectionInterface},
        document::{ContractMetadata, Erc721},
    },
    event_handlers::context::Event,
    rpc::metadata::{contract, token},
};
use color_eyre::eyre::Result;

pub async fn run(event_context: Event<'_, '_>) -> Result<()> {
    let event_data = event_context.data();

    let sender = event_data[0];
    let recipient = event_data[1];

    if sender == ZERO_FELT {
        handle_mint(event_context).await
    } else if recipient == ZERO_FELT {
        handle_burn(event_context).await
    } else {
        handle_transfer(event_context).await
    }
}

async fn handle_mint(event_context: Event<'_, '_>) -> Result<()> {
    let contract_address = event_context.contract_address();
    let block_id = event_context.block_id();
    let event_data = event_context.data();
    let db = event_context.db();
    let rpc = event_context.rpc();

    let recipient = event_data[1];
    let token_id = CairoUint256::new(event_data[2], event_data[3]);

    let erc721_collection = db.collection::<Erc721>("erc721_tokens");
    let contract_metadata_collection = db.collection::<ContractMetadata>("contract_metadata");

    let token_uri = token::get_token_uri(contract_address, block_id, rpc, token_id).await;
    let metadata = token::get_token_metadata(&token_uri).await?;
    let erc721_token = Erc721::new(contract_address, token_id, recipient, token_uri, metadata);

    erc721_collection.insert_erc721(erc721_token).await?;

    let metadata_exists =
        contract_metadata_collection.contract_metadata_exists(contract_address).await?;

    if !metadata_exists {
        let name = contract::get_name(contract_address, block_id, rpc).await;
        let symbol = contract::get_symbol(contract_address, block_id, rpc).await;
        let contract_metadata = ContractMetadata::new(contract_address, name, symbol);
        contract_metadata_collection.insert_contract_metadata(contract_metadata).await?;
    }

    Ok(())
}

async fn handle_burn(event_context: Event<'_, '_>) -> Result<()> {
    let contract_address = event_context.contract_address();
    let block_number = event_context.block_number();
    let event_data = event_context.data();
    let db = event_context.db();

    let sender = event_data[0];
    let recipient = ZERO_FELT;
    let token_id = CairoUint256::new(event_data[2], event_data[3]);

    let erc721_collection = db.collection::<Erc721>("erc721_tokens");

    erc721_collection
        .update_erc721_owner(contract_address, token_id, sender, recipient, block_number)
        .await
}

async fn handle_transfer(event_context: Event<'_, '_>) -> Result<()> {
    let contract_address = event_context.contract_address();
    let block_number = event_context.block_number();
    let event_data = event_context.data();
    let db = event_context.db();

    let sender = event_data[0];
    let recipient = event_data[1];
    let token_id = CairoUint256::new(event_data[2], event_data[3]);

    let erc721_collection = db.collection::<Erc721>("erc721_tokens");

    erc721_collection
        .update_erc721_owner(contract_address, token_id, sender, recipient, block_number)
        .await
}
