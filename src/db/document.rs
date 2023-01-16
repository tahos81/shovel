use mongodb::bson::doc;
use serde::{self, Deserialize, Serialize};
use starknet::core::types::FieldElement;

use crate::common::cairo_types::CairoUint256;

#[derive(Debug, Deserialize, Serialize)]
pub struct AddressAtBlock {
    pub address: FieldElement,
    pub block: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Contract {
    pub _id: FieldElement,
    pub name: String,
    pub symbol: String,
}

impl Contract {
    pub fn new(contract_address: FieldElement, name: String, symbol: String) -> Self {
        Self { _id: contract_address, name, symbol }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Erc721Id {
    pub contract_address: FieldElement,
    pub token_id: CairoUint256,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Erc721 {
    pub _id: Erc721Id,
    pub owner: FieldElement,
    pub previous_owners: Vec<AddressAtBlock>,
    pub token_uri: Option<String>,
}

impl Erc721 {
    pub fn new(
        contract_address: FieldElement,
        token_id: CairoUint256,
        owner: FieldElement,
        token_uri: Option<String>,
    ) -> Self {
        Self {
            _id: Erc721Id { contract_address, token_id },
            owner,
            previous_owners: vec![],
            token_uri,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Erc1155MetadataId {
    pub contract_address: FieldElement,
    pub token_id: CairoUint256,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Erc1155Metadata {
    pub _id: Erc1155MetadataId,
    pub token_uri: Option<String>,
}

#[allow(unused)]
impl Erc1155Metadata {
    pub fn new(
        contract_address: FieldElement,
        token_id: CairoUint256,
        token_uri: Option<String>,
    ) -> Self {
        Self { _id: Erc1155MetadataId { contract_address, token_id }, token_uri }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Erc1155BalanceId {
    pub contract_address: FieldElement,
    pub token_id: CairoUint256,
    pub owner: FieldElement,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Erc1155Balance {
    pub _id: Erc1155BalanceId,
    pub balance: CairoUint256,
}

impl Erc1155Balance {
    pub fn new(
        contract_address: FieldElement,
        token_id: CairoUint256,
        owner: FieldElement,
        balance: CairoUint256,
    ) -> Self {
        Self { _id: Erc1155BalanceId { contract_address, token_id, owner }, balance }
    }
}
