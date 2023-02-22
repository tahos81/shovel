use crate::{
    common::types::CairoUint256,
    events::context::Event,
    rpc::metadata::{
        contract,
        token::{self, get_token_metadata},
    },
};
use color_eyre::eyre::Result;
use sqlx::{Pool, Postgres};
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{models::BlockId, HttpTransport, JsonRpcClient},
};

pub async fn run<Database>(event_context: &Event<'_, '_, Database>) -> Result<()> {
    let contract_address = event_context.contract_address();
    let block_id = event_context.block_id();
    let block_number = event_context.block_number();
    let event_data = event_context.data();
    let db = event_context.db();
    let rpc = event_context.rpc();

    let sender = event_data[1];
    let recipient = event_data[2];
    let token_id = CairoUint256::new(event_data[3], event_data[4]);
    let amount = CairoUint256::new(event_data[5], event_data[6]);

    handle_transfer(
        contract_address,
        &block_id,
        block_number,
        sender,
        recipient,
        token_id,
        amount,
        db,
        rpc,
    )
    .await
}

pub async fn handle_transfer<Database>(
    contract_address: FieldElement,
    block_id: &BlockId,
    block_number: u64,
    sender: FieldElement,
    recipient: FieldElement,
    token_id: CairoUint256,
    amount: CairoUint256,
    pool: &Pool<Database>,
    rpc: &JsonRpcClient<HttpTransport>,
) -> Result<()> {
    // Update from balance
    if sender == FieldElement::ZERO {
        // Check if contract metadata exists
        // TODO: Write query and change to query!
        let row: (bool,) =
            sqlx::query_as("").fetch_one(pool).await.expect("Check contract metadata");
        let contract_metadata_exists = row.0;

        if !contract_metadata_exists {
            let name = contract::get_name(contract_address, block_id, rpc).await;
            let symbol = contract::get_symbol(contract_address, block_id, rpc).await;

            // TODO: Add contract metadata
            sqlx::query_as("").fetch_one(pool).await.expect("Add contract metadata");
        }

        // Check if token metadata exists
        let token_metadata_exists = erc1155_metadata_collection
            .erc1155_metadata_exists(contract_address, token_id, session)
            .await?;

        if !token_metadata_exists {
            let token_uri = token::get_erc1155_uri(contract_address, block_id, rpc, token_id).await;
            let metadata_result = get_token_metadata(&token_uri).await;
            let metadata = match metadata_result {
                Ok(metadata) => metadata,
                Err(_) => TokenMetadata::default(),
            };

            let erc1155_metadata =
                Erc1155Metadata::new(contract_address, token_id, token_uri, metadata, block_number);

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
            .update_erc1155_balance(
                contract_address,
                token_id,
                sender,
                new_balance,
                block_number,
                session,
            )
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
                .update_erc1155_balance(
                    contract_address,
                    token_id,
                    recipient,
                    new_balance,
                    block_number,
                    session,
                )
                .await?;
        }
        None => {
            // Do insert
            erc1155_collection
                .insert_erc1155_balance(
                    Erc1155Balance::new(contract_address, token_id, sender, amount, block_number),
                    session,
                )
                .await?;
        }
    }

    Ok(())
}
