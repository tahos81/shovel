#![warn(clippy::all, clippy::pedantic, clippy::style, rust_2018_idioms)]
#![allow(clippy::unreadable_literal)]
mod common;
mod db;
mod event_handlers;
mod rpc;
mod file_storage;

use color_eyre::eyre::Result;
use dotenv::dotenv;
use mongodb::error::UNKNOWN_TRANSACTION_COMMIT_RESULT;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let rpc = rpc::connect()?;
    let (db, mut session) = db::connect().await?;

    db::drop_collections(&db).await?;

    //first transfer event is in 1630
    let mut start_block = db::last_synced_block(&db, &mut session).await;
    let range = 30;

    while start_block < 16000 {
        session.start_transaction(None).await?;

        println!("getting events between block {} and {}", start_block, start_block + range);
        let transfer_events = rpc::get_transfer_events::run(start_block, range, &rpc).await?;
        println!("got {} events in total", transfer_events.len());

        event_handlers::handle_transfer_events(transfer_events, &rpc, &db, &mut session).await?;
        println!("events handled");

        start_block += range;
        db::update_last_synced_block(&db, start_block, &mut session).await?;

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
