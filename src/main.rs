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

    let rpc = rpc::setup_rpc()?;
    let db = db::connect().await?;

    let mut start_block = 12000;
    let range = 1;
    //loop
    while start_block < 16000 {
        println!("start block: {}", start_block);
        let transfer_events = rpc::get_transfers_between(start_block, range, &rpc).await?;
        println!("got {} events", transfer_events.len());
        event_handler::handle_transfer_events(transfer_events, &rpc, &db).await?;
        println!("events handled");
        start_block += range;
    }
    //loop
    Ok(())
}
