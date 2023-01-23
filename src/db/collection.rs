use super::document::{ContractMetadata, Erc1155Balance, Erc721};
use crate::common::cairo_types::CairoUint256;
use async_trait::async_trait;
use color_eyre::eyre::Result;
use mongodb::{bson::doc, options::UpdateOptions, Collection};
use starknet::core::types::FieldElement;

#[async_trait]
pub trait Erc721CollectionInterface {
    async fn insert_erc721(&self, erc721: Erc721) -> Result<()>;
    async fn update_erc721_owner(
        &self,
        contract_address: FieldElement,
        token_id: CairoUint256,
        old_owner: FieldElement,
        new_owner: FieldElement,
        block_number: u64,
    ) -> Result<()>;
}

#[async_trait]
pub trait Erc1155CollectionInterface {
    async fn insert_erc1155_balance(&self, erc1155_balance: Erc1155Balance) -> Result<()>;
    async fn update_erc1155_balance(
        &self,
        contract_address: FieldElement,
        token_id: CairoUint256,
        address: FieldElement,
        balance: CairoUint256,
    ) -> Result<()>;
    async fn get_erc1155_balance(
        &self,
        contract_address: FieldElement,
        token_id: CairoUint256,
        address: FieldElement,
    ) -> Result<Option<CairoUint256>>;
}

#[async_trait]
pub trait ContractMetadataCollectionInterface {
    async fn insert_contract_metadata(&self, contract: ContractMetadata) -> Result<()>;
    async fn contract_metadata_exists(&self, contract_address: FieldElement) -> Result<bool>;
}

#[async_trait]
impl Erc721CollectionInterface for Collection<Erc721> {
    async fn insert_erc721(&self, erc721: Erc721) -> Result<()> {
        println!("Inserting erc721");
        self.insert_one(erc721, None).await?;
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

        self.update_one(query, update, options).await?;
        Ok(())
    }
}

#[async_trait]
impl Erc1155CollectionInterface for Collection<Erc1155Balance> {
    async fn insert_erc1155_balance(&self, erc1155_balance: Erc1155Balance) -> Result<()> {
        println!("Inserting erc1155");
        self.insert_one(erc1155_balance, None).await?;
        Ok(())
    }

    async fn update_erc1155_balance(
        &self,
        contract_address: FieldElement,
        token_id: CairoUint256,
        address: FieldElement,
        balance: CairoUint256,
    ) -> Result<()> {
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

        self.update_one(query, update, options.clone()).await?;
        Ok(())
    }

    async fn get_erc1155_balance(
        &self,
        contract_address: FieldElement,
        token_id: CairoUint256,
        address: FieldElement,
    ) -> Result<Option<CairoUint256>> {
        let balance = self
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
}

#[async_trait]
impl ContractMetadataCollectionInterface for Collection<ContractMetadata> {
    async fn insert_contract_metadata(&self, contract: ContractMetadata) -> Result<()> {
        println!("Inserting contract");
        self.insert_one(contract, None).await?;
        Ok(())
    }

    async fn contract_metadata_exists(&self, contract_address: FieldElement) -> Result<bool> {
        let query = doc! {"_id": contract_address.to_string()};
        //TODO: use find instead of find_one
        let result = self.find_one(query, None).await?;
        Ok(result.is_some())
    }
}
