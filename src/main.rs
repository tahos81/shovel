#![warn(clippy::all, clippy::pedantic, clippy::style, rust_2018_idioms)]
#![allow(clippy::unreadable_literal)]
mod common;
mod db;
mod event_handlers;
mod rpc;

use color_eyre::eyre::Result;
use db::document::{ContractMetadata, Erc1155Balance, Erc1155Metadata, Erc721};
use dotenv::dotenv;
use mongodb::{bson::doc, Database};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let rpc = rpc::connect()?;
    let db = db::connect().await?;

    drop_collections(&db).await?;

    //first transfer event
    let mut start_block = 1630;
    let range = 10;

    while start_block < 16000 {
        println!("getting events between block {} and {}", start_block, start_block + range);
        let transfer_events = rpc::get_transfer_events::run(start_block, range, &rpc).await?;
        println!("got {} events in total", transfer_events.len());
        event_handlers::handle_transfer_events(transfer_events, &rpc, &db).await?;
        println!("events handled");
        start_block += range;
    }

    Ok(())
}

async fn drop_collections(db: &Database) -> Result<()> {
    // Drop all collections for test purposes
    db.collection::<Erc721>("erc721_tokens").delete_many(doc! {}, None).await?;
    db.collection::<Erc1155Balance>("erc1155_token_balances").delete_many(doc! {}, None).await?;
    db.collection::<Erc1155Metadata>("erc1155_metadata").delete_many(doc! {}, None).await?;
    db.collection::<ContractMetadata>("contract_metadata").delete_many(doc! {}, None).await?;
    println!("dropped all collections");

    Ok(())
}
