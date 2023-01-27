use crate::{
    common::types::CairoUint256,
    db::document::{ContractMetadata, Erc1155Balance, Erc1155Metadata},
    event_handlers::{context::Event, erc1155},
};
use color_eyre::eyre::Result;
use mongodb::Database;
use starknet::providers::jsonrpc::{models::EmittedEvent, HttpTransport, JsonRpcClient};

pub async fn run(
    event: &EmittedEvent,
    rpc: &JsonRpcClient<HttpTransport>,
    db: &Database,
) -> Result<()> {
    let sender = event.data[1];
    let recipient = event.data[2];

    // Get the length of the token ids array
    let token_length: u32 = event.data[3].try_into().unwrap();
    let token_length = token_length as usize;

    // This is index difference between token id and corresponding amount in the event data array
    let amount_delta = token_length * 2 + 1;

    // Zip token ids and amounts together
    let single_transfers = event.data[4..(3 + amount_delta)]
        .chunks(2)
        .map(|chunk| CairoUint256::new(chunk[0], chunk[1]))
        .zip(
            event.data[(4 + amount_delta)..]
                .chunks(2)
                .map(|chunk| CairoUint256::new(chunk[0], chunk[1])),
        );

    // Create the event context
    let event_context = Event::new(event, rpc, db);

    let erc1155_collection = db.collection::<Erc1155Balance>("erc1155_tokens");
    let erc1155_metadata_collection = db.collection::<Erc1155Metadata>("erc1155_metadata");
    let contract_metadata_collection = db.collection::<ContractMetadata>("contract_metadata");

    // For each token_id - amount pair, process a single transfer
    for (token_id, amount) in single_transfers {
        erc1155::transfer_single::handle_transfer(
            sender,
            recipient,
            token_id,
            amount,
            &erc1155_collection,
            &erc1155_metadata_collection,
            &contract_metadata_collection,
            &event_context,
        )
        .await?;
    }

    Ok(())
}
