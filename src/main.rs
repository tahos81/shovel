mod db;
mod rpc;

use db::document::*;
use mongodb::Collection;

#[tokio::main]
async fn main() {
    let shovel_db = db::connect().await;

    let contract_collection: Collection<Contract> = shovel_db.collection("contracts");
    let erc721_events = rpc::get_transfers().await;

    for event in erc721_events {
        let erc721_contract = Contract {
            address: event.from_address,
            name: "StarkRocks".to_string(),
            symbol: "SR".to_string(),
        };
        contract_collection
            .insert_one(erc721_contract, None)
            .await
            .unwrap();
    }
}
