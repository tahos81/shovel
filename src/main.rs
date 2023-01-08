mod db;
mod rpc;

use db::document::*;
use mongodb::Collection;

#[tokio::main]
async fn main() {
    let shovel_db = db::connect().await;

    let contract_collection: Collection<Contract> = shovel_db.collection("contracts");
    let erc721_events = rpc::get_transfers().await;
}
