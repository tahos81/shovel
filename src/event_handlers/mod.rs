use crate::{
    common::starknet_constants::{
        TRANSFER_BATCH_EVENT_KEY, TRANSFER_EVENT_KEY, TRANSFER_SINGLE_EVENT_KEY,
    },
    event_handlers,
    rpc::metadata::contract,
};
use color_eyre::eyre::Result;
use mongodb::Database;
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{
        models::{BlockId, EmittedEvent},
        HttpTransport, JsonRpcClient,
    },
};
use std::collections::HashSet;
pub mod context;
mod erc1155;
mod erc721;

pub async fn handle_transfer_events(
    events: Vec<EmittedEvent>,
    rpc: &JsonRpcClient<HttpTransport>,
    db: &Database,
) -> Result<()> {
    let mut blacklist: HashSet<FieldElement> = HashSet::new();

    for event in &events {
        if blacklist.contains(&event.from_address) {
            continue;
        }

        let contract_address = event.from_address;
        let block_id = BlockId::Number(event.block_number);

        let keys = &event.keys;
        if keys.contains(&TRANSFER_EVENT_KEY) {
            // Both ERC20 and ERC721 use the same event key
            let is_erc721 = contract::is_erc721(contract_address, &block_id, rpc).await?;

            if is_erc721 {
                event_handlers::erc721::transfer::run(event, rpc, db).await?;
            } else {
                blacklist.insert(contract_address);
            }
        } else if keys.contains(&TRANSFER_SINGLE_EVENT_KEY) {
            event_handlers::erc1155::transfer_single::run(event, rpc, db).await?;
        } else if keys.contains(&TRANSFER_BATCH_EVENT_KEY) {
            event_handlers::erc1155::transfer_batch::run(event, rpc, db).await?;
        }
    }

    Ok(())
}
