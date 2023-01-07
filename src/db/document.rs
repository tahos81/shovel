use serde::{self, Deserialize, Serialize};
use starknet::core::types::FieldElement;

#[derive(Debug, Deserialize, Serialize)]
struct AddressAtBlock {
    address: FieldElement,
    block: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Contract {
    pub address: FieldElement,
    pub name: String,
    pub symbol: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ERC721 {
    token_id: FieldElement,
    contract: Contract,
    owner: FieldElement,
    previous_owners: Vec<AddressAtBlock>,
    token_uri: String,
}
