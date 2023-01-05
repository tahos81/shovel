mod starknet_demo;

#[tokio::main]
async fn main() {
    starknet_demo::jsonrpc_get_events::run().await;
    //starknet_demo::get_transfers::run().await;
}
