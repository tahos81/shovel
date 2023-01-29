use crate::{
    common::types::CairoUint256,
    db::{
        collection::{
            ContractMetadataCollectionInterface, Erc1155CollectionInterface,
            Erc1155MetadataCollectionInterface,
        },
        document::{ContractMetadata, Erc1155Balance, Erc1155Metadata},
    },
    event_handlers::context::Event,
    rpc::metadata::{
        contract,
        token::{self, get_token_metadata},
    },
};
use color_eyre::eyre::Result;
use mongodb::{ClientSession, Collection};
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{models::BlockId, HttpTransport, JsonRpcClient},
};

pub async fn run(event_context: &Event<'_, '_>, session: &mut ClientSession) -> Result<()> {
    let contract_address = event_context.contract_address();
    let block_id = event_context.block_id();
    let event_data = event_context.data();
    let db = event_context.db();
    let rpc = event_context.rpc();

    let sender = event_data[1];
    let recipient = event_data[2];
    let token_id = CairoUint256::new(event_data[3], event_data[4]);
    let amount = CairoUint256::new(event_data[5], event_data[6]);

    let erc1155_collection = db.collection::<Erc1155Balance>("erc1155_tokens");
    let erc1155_metadata_collection = db.collection::<Erc1155Metadata>("erc1155_metadata");
    let contract_metadata_collection = db.collection::<ContractMetadata>("contract_metadata");

    handle_transfer(
        contract_address,
        &block_id,
        sender,
        recipient,
        token_id,
        amount,
        rpc,
        &erc1155_collection,
        &erc1155_metadata_collection,
        &contract_metadata_collection,
        session,
    )
    .await
}

pub async fn handle_transfer(
    contract_address: FieldElement,
    block_id: &BlockId,
    sender: FieldElement,
    recipient: FieldElement,
    token_id: CairoUint256,
    amount: CairoUint256,
    rpc: &JsonRpcClient<HttpTransport>,
    erc1155_collection: &Collection<Erc1155Balance>,
    erc1155_metadata_collection: &Collection<Erc1155Metadata>,
    contract_metadata_collection: &Collection<ContractMetadata>,
    session: &mut ClientSession,
) -> Result<()> {
    // Update from balance
    if sender == FieldElement::ZERO {
        // Check if contract metadata exists
        let contract_metadata_exists = contract_metadata_collection
            .contract_metadata_exists(contract_address, session)
            .await?;

        if !contract_metadata_exists {
            let name = contract::get_name(contract_address, block_id, rpc).await;
            let symbol = contract::get_symbol(contract_address, block_id, rpc).await;
            let contract_metadata = ContractMetadata::new(contract_address, name, symbol);
            contract_metadata_collection
                .insert_contract_metadata(contract_metadata, session)
                .await?;
        }

        // Check if token metadata exists
        let token_metadata_exists = erc1155_metadata_collection
            .erc1155_metadata_exists(contract_address, token_id, session)
            .await?;

        if !token_metadata_exists {
            let token_uri = token::get_erc1155_uri(contract_address, block_id, rpc, token_id).await;
            let metadata = get_token_metadata(&token_uri).await?;
            let erc1155_metadata =
                Erc1155Metadata::new(contract_address, token_id, token_uri, metadata);

            erc1155_metadata_collection.insert_erc1155_metadata(erc1155_metadata, session).await?;
        }
    } else {
        // We know that from balance won't be zero
        let from_balance = if let Some(balance) = erc1155_collection
            .get_erc1155_balance(contract_address, token_id, sender, session)
            .await?
        {
            balance
        } else {
            println!("Impossible state, from balance 0, using amount as default");
            amount
        };

        let new_balance = from_balance - amount;
        erc1155_collection
            .update_erc1155_balance(contract_address, token_id, sender, new_balance, session)
            .await?;
    }

    // Update to balance
    match erc1155_collection
        .get_erc1155_balance(contract_address, token_id, sender, session)
        .await?
    {
        Some(previous_balance) => {
            let new_balance = previous_balance + amount;
            erc1155_collection
                .update_erc1155_balance(contract_address, token_id, recipient, new_balance, session)
                .await?;
        }
        None => {
            // Do insert
            erc1155_collection
                .insert_erc1155_balance(
                    Erc1155Balance::new(contract_address, token_id, sender, amount),
                    session,
                )
                .await?;
        }
    }

    Ok(())
}
