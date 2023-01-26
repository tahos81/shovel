#![warn(clippy::all, clippy::pedantic, clippy::style)]
#![allow(clippy::unreadable_literal)]
mod common;
mod db;
mod event_handler;
mod rpc;

use color_eyre::eyre::Result;
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let rpc = rpc::connect()?;
    let (db, mut session) = db::connect().await?;

    let mut start_block = 14000;
    let range = 20;

    session.start_transaction(None).await?;

    while start_block < 16000 {
        println!("getting events between block {} and {}", start_block, start_block + range);
        let transfer_events = rpc::get_transfer_events::run(start_block, range, &rpc).await?;
        println!("got {} events in total", transfer_events.len());
        event_handler::handle_transfer_events(transfer_events, &rpc, &db).await?;
        println!("events handled");
        start_block += range;
    }

    session.commit_transaction().await?;

    Ok(())
}
