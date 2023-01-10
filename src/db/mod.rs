pub mod document;

use self::document::{Contract, ERC1155, ERC721};
use async_trait::async_trait;
use mongodb::{
    bson::doc, options::ClientOptions, options::UpdateOptions, Client, Collection, Database,
};
use starknet::core::types::FieldElement;
use std::env;

#[async_trait]
pub trait NftExt {
    async fn insert_contract(&self, contract: Contract);
    async fn insert_erc721(&self, erc721: ERC721);
    async fn insert_erc1155(&self, erc1155: ERC1155);
    async fn update_erc721_owner(
        &self,
        contract_address: FieldElement,
        token_id: FieldElement,
        old_owner: FieldElement,
        new_owner: FieldElement,
        block_number: u64,
    );
    async fn update_erc1155(&self, erc1155: ERC1155);
    async fn contract_exists(&self, contract_address: FieldElement) -> bool;
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

    async fn update_erc721_owner(
        &self,
        contract_address: FieldElement,
        token_id: FieldElement,
        old_owner: FieldElement,
        new_owner: FieldElement,
        block_number: u64,
    ) {
        let collection: Collection<ERC721> = self.collection("erc721_tokens");

        let query = doc! {"_id": {
            "contract_address": contract_address.to_string(),
            "token_id": token_id.to_string()
        }};

        let update = doc! {
            "$set": {
            "owner": new_owner.to_string()
            },
            "$push": {
                "previous_owners": {
                    "address": old_owner.to_string(),
                    "block": block_number as i64
                }
            }
        };

        let options = UpdateOptions::builder().upsert(true).build();

        collection.update_one(query, update, options).await.unwrap();
    }

    async fn update_erc1155(&self, erc1155: ERC1155) {
        let collection: Collection<ERC1155> = self.collection("erc1155_tokens");
        drop(erc1155);
        drop(collection);
    }

    async fn contract_exists(&self, contract_address: FieldElement) -> bool {
        let collection: Collection<Contract> = self.collection("contracts");
        let query = doc! {"_id": contract_address.to_string()};
        //TODO: use find instead of find_one
        let result = collection.find_one(query, None).await.unwrap();
        result.is_some()
    }
}

pub async fn connect() -> Database {
    let client_url_with_options =
        env::var("CLIENT_URL_WITH_OPTIONS").expect("configure your .env file");
    let client_options = ClientOptions::parse(client_url_with_options).await.unwrap();

    let client = Client::with_options(client_options).unwrap();
    client.database("shovel")
}
