mod db;
mod rpc;

#[tokio::main]
async fn main() {
    crate::rpc::get_transfers().await;
}
