pub mod document;

use async_trait::async_trait;
use dotenv::dotenv;
use mongodb::{bson::doc, options::ClientOptions, Client, Collection, Database};
use std::env;

use self::document::{Contract, ERC1155, ERC721};

#[async_trait]
pub trait NftExt {
    async fn insert_contract(&self, _: Contract) {}
    async fn insert_erc721(&self, _: ERC721) {}
    async fn insert_erc1155(&self, _: ERC1155) {}
    async fn update_erc721(&self, _: ERC721) {}
    async fn update_erc1155(&self, _: ERC1155) {}
}

#[async_trait]
impl NftExt for Database {
    async fn insert_contract(&self, contract: Contract) {
        let collection: Collection<Contract> = self.collection("contracts");
        collection.insert_one(contract, None).await.unwrap();
    }

    async fn insert_erc721(&self, erc721: ERC721) {
        let collection: Collection<ERC721> = self.collection("erc721_tokens");
        collection.insert_one(erc721, None).await.unwrap();
    }

    async fn insert_erc1155(&self, erc1155: ERC1155) {
        let collection: Collection<ERC1155> = self.collection("erc1155_tokens");
        collection.insert_one(erc1155, None).await.unwrap();
    }

    async fn update_erc721(&self, erc721: ERC721) {
        let collection: Collection<ERC721> = self.collection("erc721_tokens");
    }

    async fn update_erc1155(&self, erc1155: ERC1155) {
        let collection: Collection<ERC1155> = self.collection("erc1155_tokens");
    }
}

pub async fn connect() -> Database {
    dotenv().ok();

    let client_url_with_options =
        env::var("CLIENT_URL_WITH_OPTIONS").expect("configure your .env file");
    let client_options = ClientOptions::parse(client_url_with_options).await.unwrap();

    let client = Client::with_options(client_options).unwrap();
    client.database("shovel")
}
