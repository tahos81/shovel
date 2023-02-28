pub mod erc1155;
pub mod erc721;

use color_eyre::eyre::Result;
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{
        models::{BlockId, EmittedEvent},
        HttpTransport, JsonRpcClient,
    },
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
        let block_id = BlockId::Number(event.block_number);
        let contract_address = event.from_address;

        if blacklist.contains(&event.from_address) {
            continue;
        }

        let keys = &event.keys;
        if keys.contains(&TRANSFER_EVENT_KEY) {
            // Both ERC20 and ERC721 use the same event key
            let is_erc721 = contract::is_erc721(contract_address, &block_id, rpc).await?;

            if is_erc721 {
                erc721::transfer::run(event, rpc, transaction).await?;
            } else {
                blacklist.insert(contract_address);
            }
        } else if keys.contains(&TRANSFER_SINGLE_EVENT_KEY) {
            erc1155::transfer_single::run(event, rpc, transaction).await?;
        } else if keys.contains(&TRANSFER_BATCH_EVENT_KEY) {
            erc1155::transfer_batch::run(event, rpc, transaction).await?;
        }
    }

    Ok(())
}
