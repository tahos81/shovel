mod db;
mod event_handler;
mod rpc;

use dotenv::dotenv;

//use starknet::macros::felt;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let db = db::connect().await;
    let rpc = rpc::setup_rpc().await;
    let transfer_events = rpc::get_transfers_between(18545, 5, &rpc).await;
    event_handler::handle_transfer_events(transfer_events, &rpc, &db).await;
}
