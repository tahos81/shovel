use crate::common::types::CairoUint256;
use mongodb::bson::doc;
use serde::{self, Deserialize, Serialize};
use serde_json::Number;
use starknet::core::types::FieldElement;

#[derive(Debug, Deserialize, Serialize)]
struct AddressAtBlock {
    address: FieldElement,
    block: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ContractMetadata {
    _id: FieldElement,
    name: String,
    symbol: String,
    last_updated: u64,
}

impl ContractMetadata {
    pub fn new(
        contract_address: FieldElement,
        name: String,
        symbol: String,
        last_updated: u64,
    ) -> Self {
        Self { _id: contract_address, name, symbol, last_updated }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Erc721Id {
    pub contract_address: FieldElement,
    pub token_id: CairoUint256,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Erc721 {
    _id: Erc721Id,
    owner: FieldElement,
    previous_owners: Vec<AddressAtBlock>,
    token_uri: String,
    metadata: TokenMetadata,
    last_updated: u64,
}

impl Erc721 {
    pub fn new(
        contract_address: FieldElement,
        token_id: CairoUint256,
        owner: FieldElement,
        token_uri: String,
        metadata: TokenMetadata,
        last_updated: u64,
    ) -> Self {
        Self {
            _id: Erc721Id { contract_address, token_id },
            owner,
            previous_owners: vec![],
            token_uri,
            metadata,
            last_updated,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Erc1155MetadataId {
    contract_address: FieldElement,
    token_id: CairoUint256,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Erc1155Metadata {
    _id: Erc1155MetadataId,
    token_uri: String,
    metadata: TokenMetadata,
    last_updated: u64,
}

#[allow(unused)]
impl Erc1155Metadata {
    pub fn new(
        contract_address: FieldElement,
        token_id: CairoUint256,
        token_uri: String,
        metadata: TokenMetadata,
        last_updated: u64,
    ) -> Self {
        Self {
            _id: Erc1155MetadataId { contract_address, token_id },
            token_uri,
            metadata,
            last_updated,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Erc1155BalanceId {
    contract_address: FieldElement,
    token_id: CairoUint256,
    owner: FieldElement,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Erc1155Balance {
    _id: Erc1155BalanceId,
    balance: CairoUint256,
    last_updated: u64,
}

impl Erc1155Balance {
    pub fn new(
        contract_address: FieldElement,
        token_id: CairoUint256,
        owner: FieldElement,
        balance: CairoUint256,
        last_updated: u64,
    ) -> Self {
        Self { _id: Erc1155BalanceId { contract_address, token_id, owner }, balance, last_updated }
    }

    pub fn balance(&self) -> CairoUint256 {
        self.balance
    }
}

pub enum MetadataType<'a> {
    Http(&'a str),
    Ipfs(&'a str),
    OnChain(&'a str),
}

#[derive(Debug, Deserialize, Serialize)]
enum DisplayType {
    Number,
    BoostPercentage,
    BoostNumber,
    Date,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum AttributeValue {
    String(String),
    Number(Number),
}

#[derive(Debug, Deserialize, Serialize)]
struct Attribute {
    display_type: Option<DisplayType>,
    trait_type: Option<String>,
    value: AttributeValue,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TokenMetadata {
    image: Option<String>,
    image_data: Option<String>,
    external_url: Option<String>,
    description: Option<String>,
    name: Option<String>,
    attributes: Option<Vec<Attribute>>,
    background_color: Option<String>,
    animation_url: Option<String>,
    youtube_url: Option<String>,
}

impl TokenMetadata {
    pub const EMPTY: Self = Self {
        name: None,
        description: None,
        image: None,
        image_data: None,
        external_url: None,
        animation_url: None,
        attributes: None,
        background_color: None,
        youtube_url: None,
    };
}
