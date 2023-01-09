mod db;
mod event_handler;
mod rpc;

//use starknet::macros::felt;

#[tokio::main]
async fn main() {
    rpc::get_transfers_between(5, 5).await;
}
