#![warn(clippy::all, clippy::pedantic, clippy::style, rust_2018_idioms)]
#![allow(clippy::unreadable_literal)]
mod common;
mod db;
mod event_handlers;
mod rpc;

use color_eyre::eyre::Result;
use db::document::{ContractMetadata, Erc1155Balance, Erc1155Metadata, Erc721};
use dotenv::dotenv;
use mongodb::{bson::doc, error::UNKNOWN_TRANSACTION_COMMIT_RESULT, Database};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let rpc = rpc::connect()?;
    let (db, mut session) = db::connect().await?;

    drop_collections(&db).await?;

    //first transfer event is in 1630
    let mut start_block = 1930;
    let range = 20;

    while start_block < 16000 {
        session.start_transaction(None).await?;

        println!("getting events between block {} and {}", start_block, start_block + range);
        let transfer_events = rpc::get_transfer_events::run(start_block, range, &rpc).await?;
        println!("got {} events in total", transfer_events.len());
        event_handlers::handle_transfer_events(transfer_events, &rpc, &db, &mut session).await?;
        println!("events handled");
        start_block += range;

        loop {
            let result = session.commit_transaction().await;
            if let Err(ref error) = result {
                if error.contains_label(UNKNOWN_TRANSACTION_COMMIT_RESULT) {
                    continue;
                }
            }
            break result?;
        }
    }

    Ok(())
}

async fn drop_collections(db: &Database) -> Result<()> {
    // Drop all collections for test purposes
    db.collection::<Erc721>("erc721_tokens").delete_many(doc! {}, None).await?;
    db.collection::<Erc1155Balance>("erc1155_tokens").delete_many(doc! {}, None).await?;
    db.collection::<Erc1155Metadata>("erc1155_metadata").delete_many(doc! {}, None).await?;
    db.collection::<ContractMetadata>("contract_metadata").delete_many(doc! {}, None).await?;
    println!("dropped all collections");

    Ok(())
}
