pub mod document;

use dotenv::dotenv;
use mongodb::{options::ClientOptions, Client, Database};
use std::env;

pub async fn connect() -> Database {
    dotenv().ok();

    let client_url_with_options =
        env::var("CLIENT_URL_WITH_OPTIONS").expect("configure your .env file");
    let client_options = ClientOptions::parse(client_url_with_options).await.unwrap();

    let client = Client::with_options(client_options).unwrap();
    client.database("shovel")
}
