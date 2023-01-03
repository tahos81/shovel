mod starknet_demo;

#[tokio::main]
async fn main() {
    starknet_demo::get_transfers::run().await;
}