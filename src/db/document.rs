use mongodb::bson::doc;
use serde::{self, Deserialize, Serialize};
use starknet::{core::types::FieldElement, providers::jsonrpc::models::EmittedEvent};

#[derive(Debug, Deserialize, Serialize)]
pub struct AddressAtBlock {
    address: FieldElement,
    block: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Erc721ID {
    pub token_id: FieldElement,
    pub contract_address: FieldElement,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Contract {
    pub _id: FieldElement,
    pub name: Option<String>,
    pub symbol: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ERC721 {
    pub _id: Erc721ID,
    pub owner: FieldElement,
    pub previous_owners: Option<Vec<AddressAtBlock>>,
    pub token_uri: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ERC1155 {
    token_id: FieldElement,
    amount: FieldElement,
    contract: Contract,
    owner: FieldElement,
    uri: String,
}

impl From<&EmittedEvent> for Contract {
    fn from(event: &EmittedEvent) -> Self {
        Self {
            _id: event.from_address,
            name: None,
            symbol: None,
        }
    }
}
