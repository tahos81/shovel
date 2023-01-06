mod starknet_demo;

use ::dotenv::dotenv;
use mongodb::{bson::Document, options::ClientOptions, *};
use std::env;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let client_url_with_options =
        env::var("CLIENT_URL_WITH_OPTIONS").expect("configure your .env file");
    let client_options = ClientOptions::parse(client_url_with_options).await.unwrap();

    let client = Client::with_options(client_options).unwrap();
    let shovel_db = client.database("shovel");

    let erc721_collection: Collection<Document> = shovel_db.collection("erc721_tokens");
    let erc1155_collection: Collection<Document> = shovel_db.collection("erc1155_tokens");
    let contract_collection: Collection<Document> = shovel_db.collection("contracts");
    starknet_demo::jsonrpc_get_events::run().await;
}
