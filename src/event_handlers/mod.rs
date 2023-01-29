pub mod context;
mod erc1155;
mod erc721;

use crate::{
    common::starknet_constants::{
        TRANSFER_BATCH_EVENT_KEY, TRANSFER_EVENT_KEY, TRANSFER_SINGLE_EVENT_KEY,
    },
    event_handlers,
    rpc::metadata::contract,
};
use color_eyre::eyre::Result;
use context::Event;
use mongodb::{ClientSession, Database};
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{models::EmittedEvent, HttpTransport, JsonRpcClient},
};
use std::collections::HashSet;

pub async fn handle_transfer_events(
    events: Vec<EmittedEvent>,
    rpc: &JsonRpcClient<HttpTransport>,
    db: &Database,
    session: &mut ClientSession,
) -> Result<()> {
    let mut blacklist: HashSet<FieldElement> = HashSet::new();

    for event in &events {
        if blacklist.contains(&event.from_address) {
            continue;
        }

        let event_context = Event::new(event, rpc, db);

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
                event_handlers::erc721::transfer::run(&event_context, session).await?;
            } else {
                blacklist.insert(event_context.contract_address());
            }
        } else if keys.contains(&TRANSFER_SINGLE_EVENT_KEY) {
            event_handlers::erc1155::transfer_single::run(&event_context, session).await?;
        } else if keys.contains(&TRANSFER_BATCH_EVENT_KEY) {
            event_handlers::erc1155::transfer_batch::run(&event_context, session).await?;
        }
    }

    Ok(())
}
