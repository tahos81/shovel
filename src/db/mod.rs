pub mod document;

use crate::common::{cairo_types::CairoUint256, errors::ConfigError};

use self::document::{Contract, ERC1155Balance, ERC721};
use async_trait::async_trait;
use color_eyre::eyre::Result;
use mongodb::{
    bson::doc, options::ClientOptions, options::UpdateOptions, Client, Collection, Database,
};
use starknet::core::types::FieldElement;
use std::env;

#[async_trait]
pub trait NftExt {
    async fn get_erc1155_balance(
        &self,
        contract_address: FieldElement,
        token_id: CairoUint256,
        address: FieldElement,
    ) -> Result<Option<CairoUint256>>;
    async fn insert_contract(&self, contract: Contract) -> Result<()>;
    async fn insert_erc721(&self, erc721: ERC721) -> Result<()>;
    async fn insert_erc1155_balance(&self, erc1155_balance: ERC1155Balance) -> Result<()>;
    async fn update_erc721_owner(
        &self,
        contract_address: FieldElement,
        token_id: CairoUint256,
        old_owner: FieldElement,
        new_owner: FieldElement,
        block_number: u64,
    ) -> Result<()>;
    async fn update_erc1155_balance(
        &self,
        contract_address: FieldElement,
        token_id: CairoUint256,
        address: FieldElement,
        balance: CairoUint256,
    ) -> Result<()>;
    async fn contract_exists(&self, contract_address: FieldElement) -> Result<bool>;
}

#[async_trait]
impl NftExt for Database {
    async fn get_erc1155_balance(
        &self,
        contract_address: FieldElement,
        token_id: CairoUint256,
        address: FieldElement,
    ) -> Result<Option<CairoUint256>> {
        let balance = self
            .collection::<ERC1155Balance>("erc1155_token_balances")
            .find_one(
                doc! {
                    "_id.contract_address": contract_address.to_string(),
                    "_id.token_id.low": token_id.low.to_string(),
                    "_id.token_id.high": token_id.high.to_string(),
                    "_id.owner": address.to_string(),
                },
                None,
            )
            .await?
            .map(|response| response.balance);
        Ok(balance)
    }

    async fn insert_contract(&self, contract: Contract) -> Result<()> {
        let collection: Collection<Contract> = self.collection("contracts");
        println!("Inserting contract");
        collection.insert_one(contract, None).await?;
        Ok(())
    }

    async fn insert_erc721(&self, erc721: ERC721) -> Result<()> {
        let collection: Collection<ERC721> = self.collection("erc721_tokens");
        println!("Inserting erc721");
        collection.insert_one(erc721, None).await?;
        Ok(())
    }

    async fn insert_erc1155_balance(&self, erc1155_balance: ERC1155Balance) -> Result<()> {
        let collection: Collection<ERC1155Balance> = self.collection("erc1155_token_balances");
        println!("Inserting erc1155");
        collection.insert_one(erc1155_balance, None).await?;
        Ok(())
    }

    async fn update_erc721_owner(
        &self,
        contract_address: FieldElement,
        token_id: CairoUint256,
        old_owner: FieldElement,
        new_owner: FieldElement,
        block_number: u64,
    ) -> Result<()> {
        let collection: Collection<ERC721> = self.collection("erc721_tokens");

        let query = doc! {"_id": {
            "contract_address": contract_address.to_string(),
            "token_id": {
                "low": token_id.low.to_string(),
                "high": token_id.high.to_string()
            }
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
        println!("Updating erc721 owner");

        collection.update_one(query, update, options).await?;
        Ok(())
    }

    async fn update_erc1155_balance(
        &self,
        contract_address: FieldElement,
        token_id: CairoUint256,
        address: FieldElement,
        balance: CairoUint256,
    ) -> Result<()> {
        let collection: Collection<ERC1155Balance> = self.collection("erc1155_token_balances");

        let query = doc! {"_id": {
            "contract_address": contract_address.to_string(),
            "token_id": {
                "low": token_id.low.to_string(),
                "high": token_id.high.to_string()
            },
            "owner": address.to_string()
        }};

        let update = doc! {
            "$set": {
                "balance": {
                    "low": balance.low.to_string(),
                    "high": balance.high.to_string()
                }
            }
        };

        let options = UpdateOptions::builder().upsert(true).build();
        println!("Updating erc1155 balance");

        collection.update_one(query, update, options.clone()).await?;
        Ok(())
    }

    async fn contract_exists(&self, contract_address: FieldElement) -> Result<bool> {
        let collection: Collection<Contract> = self.collection("contracts");
        let query = doc! {"_id": contract_address.to_string()};
        //TODO: use find instead of find_one
        let result = collection.find_one(query, None).await?;
        Ok(result.is_some())
    }
}

pub async fn connect() -> Result<Database, ConfigError> {
    let client_url_with_options = env::var("CONNECTION_STRING_WITH_OPTIONS")?;
    let client_options = ClientOptions::parse(client_url_with_options).await?;

    let client = Client::with_options(client_options)?;
    Ok(client.database("shovel"))
}
