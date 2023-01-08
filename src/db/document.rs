use serde::{self, Deserialize, Serialize};
use starknet::{core::types::FieldElement, providers::jsonrpc::models::EmittedEvent};

#[derive(Debug, Deserialize, Serialize)]
struct AddressAtBlock {
    address: FieldElement,
    block: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Contract {
    pub address: FieldElement,
    pub name: Option<String>,
    pub symbol: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ERC721 {
    token_id: FieldElement,
    contract: Contract,
    owner: FieldElement,
    previous_owners: Vec<AddressAtBlock>,
    token_uri: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ERC1155 {
    token_id: FieldElement,
    amount: FieldElement,
    contract: Contract,
    owner: FieldElement,
    uri: String,
}

impl From<EmittedEvent> for Contract {
    fn from(event: EmittedEvent) -> Self {
        Self {
            address: event.from_address,
            name: None,
            symbol: None,
        }
    }
}

// TODO: implement From trait for ERC721 and ERC1155
