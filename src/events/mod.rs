pub mod context;
pub mod erc1155;
pub mod erc721;

use color_eyre::eyre::Result;
use context::Event;
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{models::EmittedEvent, HttpTransport, JsonRpcClient},
};
use std::collections::HashSet;

use crate::{
    common::starknet_constants::{
        TRANSFER_BATCH_EVENT_KEY, TRANSFER_EVENT_KEY, TRANSFER_SINGLE_EVENT_KEY,
    },
    rpc::metadata::contract,
};

pub async fn handle_transfer_events<'ctx>(
    events: Vec<EmittedEvent>,
    rpc: &'ctx JsonRpcClient<HttpTransport>,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<()> {
    let mut blacklist: HashSet<FieldElement> = HashSet::new();

    for event in &events {
        if blacklist.contains(&event.from_address) {
            continue;
        }

        let event_context = Event::new(event, rpc);

        let keys = &event.keys;
        if keys.contains(&TRANSFER_EVENT_KEY) {
            // Both ERC20 and ERC721 use the same event key
            let is_erc721 = contract::is_erc721(
                event_context.contract_address(),
                &event_context.block_id(),
                rpc,
            )
            .await?;

            if is_erc721 {
                erc721::transfer::run(&event_context, transaction).await?;
            } else {
                blacklist.insert(event_context.contract_address());
            }
        } else if keys.contains(&TRANSFER_SINGLE_EVENT_KEY) {
            erc1155::transfer_single::run(&event_context, transaction).await?;
        } else if keys.contains(&TRANSFER_BATCH_EVENT_KEY) {
            erc1155::transfer_batch::run(&event_context, transaction).await?;
        }
    }

    Ok(())
}
