#![warn(clippy::all, clippy::pedantic, clippy::style)]
#![allow(clippy::unreadable_literal)]
mod common;
mod db;
mod event_handlers;
mod rpc;

use color_eyre::eyre::Result;
use db::document::{ContractMetadata, Erc1155Balance, Erc1155Metadata, Erc721};
use dotenv::dotenv;
use mongodb::bson::doc;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let rpc = rpc::connect()?;
    let db = db::connect().await?;

    // Drop all collections for test purposes
    db.collection::<Erc721>("erc721_tokens").delete_many(doc! {}, None).await?;
    db.collection::<Erc1155Balance>("erc1155_token_balances").delete_many(doc! {}, None).await?;
    db.collection::<Erc1155Metadata>("erc1155_metadata").delete_many(doc! {}, None).await?;
    db.collection::<ContractMetadata>("contract_metadata").delete_many(doc! {}, None).await?;
    println!("dropped all collections");

    let mut start_block = 14020;
    let range = 20;

    while start_block < 16000 {
        println!("getting events between block {} and {}", start_block, start_block + range);
        let transfer_events = rpc::get_transfers_between(start_block, range, &rpc).await?;
        println!("got {} events in total", transfer_events.len());
        event_handlers::handle_transfer_events(transfer_events, &rpc, &db).await?;
        println!("events handled");
        start_block += range;
    }

    Ok(())
}
