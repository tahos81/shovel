mod starknet_demo;

use ::dotenv::dotenv;
use mongodb::{options::ClientOptions, *};
use serde::{self, Deserialize, Serialize};
use starknet::{core::types::FieldElement, providers::jsonrpc::models::BlockId};
use std::env;

#[tokio::main]
async fn main() {
    pub struct AddressAtBlock {
        address: FieldElement,
        block: BlockId,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct Contract {
        address: FieldElement,
        name: String,
        symbol: String,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct ERC721 {
        token_id: FieldElement,
        contract: Contract,
        owner: FieldElement,
        previous_owners: Vec<FieldElement>, // switch to AddressAtBlock
        token_uri: String,
    }

    dotenv().ok();

    let client_url_with_options =
        env::var("CLIENT_URL_WITH_OPTIONS").expect("configure your .env file");
    let client_options = ClientOptions::parse(client_url_with_options).await.unwrap();

    let client = Client::with_options(client_options).unwrap();
    let shovel_db = client.database("shovel");

    //let erc721_collection: Collection<ERC721> = shovel_db.collection("erc721_tokens");
    //let erc1155_collection: Collection<Document> = shovel_db.collection("erc1155_tokens");
    let contract_collection: Collection<Contract> = shovel_db.collection("contracts");
    let erc721_events = starknet_demo::jsonrpc_get_events::run().await;

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
