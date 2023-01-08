pub mod document;

use dotenv::dotenv;
use mongodb::{options::ClientOptions, Client, Collection, Database};
use std::env;

use self::document::ERC721;

pub async fn connect() -> Database {
    dotenv().ok();

    let client_url_with_options =
        env::var("CLIENT_URL_WITH_OPTIONS").expect("configure your .env file");
    let client_options = ClientOptions::parse(client_url_with_options).await.unwrap();

    let client = Client::with_options(client_options).unwrap();
    client.database("shovel")
}

pub async fn write_erc721(db: Database, erc721: ERC721) {
    let collection: Collection<ERC721> = db.collection("erc721_tokens");
    collection.insert_one(erc721, None).await.unwrap();
}
