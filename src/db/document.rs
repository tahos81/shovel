use mongodb::bson::doc;
use serde::{self, Deserialize, Serialize};
use starknet::core::types::FieldElement;

#[derive(Debug, Deserialize, Serialize)]
pub struct AddressAtBlock {
    address: FieldElement,
    block: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Erc721ID {
    pub contract_address: FieldElement,
    pub token_id: FieldElement,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Contract {
    pub _id: FieldElement,
    pub name: String,
    pub symbol: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ERC721 {
    pub _id: Erc721ID,
    pub owner: FieldElement,
    pub previous_owners: Vec<AddressAtBlock>,
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

impl Contract {
    pub fn new(contract_address: FieldElement, name: String, symbol: String) -> Self {
        Self {
            _id: contract_address,
            name,
            symbol,
        }
    }
}

impl ERC721 {
    pub fn new(
        contract_address: FieldElement,
        token_id: FieldElement,
        owner: FieldElement,
        token_uri: String,
    ) -> Self {
        Self {
            _id: Erc721ID {
                contract_address,
                token_id,
            },
            owner,
            previous_owners: vec![],
            token_uri,
        }
    }
}
