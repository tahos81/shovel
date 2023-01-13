#![warn(clippy::all, clippy::pedantic, clippy::style)]
#![allow(clippy::unreadable_literal)]
mod common;
mod db;
mod event_handler;
mod rpc;

use dotenv::dotenv;
use eyre::Result;

//use starknet::macros::felt;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let rpc = rpc::setup_rpc()?;
    let db = db::connect().await;
    let transfer_events = rpc::get_transfers_between(14791, 0, &rpc).await;
    event_handler::handle_transfer_events(transfer_events, &rpc, &db).await;
    Ok(())
}
