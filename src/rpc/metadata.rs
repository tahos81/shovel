pub mod token {
    use crate::common::{
        starknet_constants::{TOKEN_URI_SELECTOR, ZERO_FELT},
        traits::ToUtf8String,
        types::CairoUint256,
    };
    use base64::{engine::general_purpose, Engine as _};
    use color_eyre::eyre;
    use reqwest::Client;
    use serde::{Deserialize, Serialize};
    use serde_json::Number;
    use starknet::{
        core::types::{BlockId, FieldElement, FunctionCall},
        macros::selector,
        providers::{
            jsonrpc::{HttpTransport, JsonRpcClient},
            Provider,
        },
    };
    use std::env;
    use urlencoding;

    pub enum MetadataType<'a> {
        Http(&'a str),
        Ipfs(&'a str),
        OnChain(&'a str),
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub enum DisplayType {
        Number,
        BoostPercentage,
        BoostNumber,
        Date,
    }

    #[derive(Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum AttributeValue {
        String(String),
        Number(Number),
        Bool(bool),
        StringVec(Vec<String>),
        NumberVec(Vec<Number>),
        BoolVec(Vec<bool>),
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Attribute {
        pub display_type: Option<DisplayType>,
        pub trait_type: Option<String>,
        pub value: AttributeValue,
    }

    #[derive(Debug, Default, Deserialize, Serialize)]
    pub struct TokenMetadata {
        pub image: Option<String>,
        pub image_data: Option<String>,
        pub external_url: Option<String>,
        pub description: Option<String>,
        pub name: Option<String>,
        pub attributes: Option<Vec<Attribute>>,
        pub background_color: Option<String>,
        pub animation_url: Option<String>,
        pub youtube_url: Option<String>,
    }

    /// Gets the token URI for a given token ID
    pub async fn get_erc721_uri(
        address: FieldElement,
        block_id: &BlockId,
        rpc: &JsonRpcClient<HttpTransport>,
        token_id: CairoUint256,
    ) -> String {
        // token_uri(uint256) | tokenURI(uint256)
        // uint256 -> [ felt, felt ]
        let request = FunctionCall {
            contract_address: address,
            entry_point_selector: TOKEN_URI_SELECTOR,
            calldata: vec![token_id.low, token_id.high],
        };

        let Ok(token_uri_response) = rpc.call(request, block_id).await else {
            return String::new();
        };

        // If tokenURI function is EIP721Metadata compliant, it should return one felt
        // Otherwise we also consider the case where contracts returns a felt array
        let is_felt_array = token_uri_response.len() > 1;

        if is_felt_array {
            token_uri_response.to_utf8_string()
        } else {
            token_uri_response.get(0).unwrap_or(&ZERO_FELT).to_utf8_string()
        }
    }

    pub async fn get_erc1155_uri(
        address: FieldElement,
        block_id: &BlockId,
        rpc: &JsonRpcClient<HttpTransport>,
        token_id: CairoUint256,
    ) -> String {
        let possible_selectors =
            vec![selector!("uri"), selector!("tokenURI"), selector!("token_uri")];

        let mut token_uri_response: Option<Vec<FieldElement>> = None;

        for selector in possible_selectors {
            let request = FunctionCall {
                contract_address: address,
                entry_point_selector: selector,
                calldata: vec![token_id.low, token_id.high],
            };
            match rpc.call(request, block_id).await {
                Ok(felt_array) => {
                    token_uri_response = Some(felt_array);
                    break;
                }
                Err(e) => {
                    // dbg!(e);
                }
            };
        }

        let Some(token_uri_response) = token_uri_response else {
            return String::new();
        };

        let is_felt_array = token_uri_response.len() > 1;

        if is_felt_array {
            token_uri_response.to_utf8_string()
        } else {
            token_uri_response.get(0).unwrap_or(&ZERO_FELT).to_utf8_string()
        }
    }

    /// Gets token metadata for a given uri
    pub async fn get_token_metadata(uri: &str) -> eyre::Result<TokenMetadata> {
        let client = Client::new();

        let metadata_type = get_metadata_type(uri);

        match metadata_type {
            MetadataType::Ipfs(uri) => Ok(get_ipfs_metadata(uri, &client).await?),
            MetadataType::Http(uri) => Ok(get_http_metadata(uri, &client).await?),
            MetadataType::OnChain(uri) => Ok(get_onchain_metadata(uri)?),
        }
    }

    async fn get_ipfs_metadata(uri: &str, client: &Client) -> eyre::Result<TokenMetadata> {
        let username = env::var("IPFS_USERNAME")?;
        let password = env::var("IPFS_PASSWORD")?;

        let mut ipfs_url = "https://ipfs.infura.io:5001/api/v0/cat?arg=".to_string();
        let ipfs_hash = uri.trim_start_matches("ipfs://");
        ipfs_url.push_str(ipfs_hash);

        let req = client.post(ipfs_url).basic_auth(&username, Some(&password));
        let resp = req.send().await?;
        let metadata: TokenMetadata = resp.json().await?;
        Ok(metadata)
    }

    async fn get_http_metadata(uri: &str, client: &Client) -> eyre::Result<TokenMetadata> {
        let resp = client.get(uri).send().await?;
        let metadata: TokenMetadata = resp.json().await?;
        Ok(metadata)
    }

    fn get_onchain_metadata(uri: &str) -> eyre::Result<TokenMetadata> {
        // Try to split from the comma as it is the standard with on chain metadata
        let url_encoded = urlencoding::decode(uri).map(|s| String::from(s.as_ref()));
        let uri_string = match url_encoded {
            Ok(encoded) => encoded,
            Err(_) => String::from(uri),
        };

        match uri_string.split_once(',') {
            Some(("data:application/json;base64", uri)) => {
                // If it is base64 encoded, decode it, parse and return
                let decoded = general_purpose::STANDARD.decode(uri)?;
                let decoded = std::str::from_utf8(&decoded)?;
                let metadata: TokenMetadata = serde_json::from_str(decoded)?;
                Ok(metadata)
            }
            Some(("data:application/json", uri)) => {
                // If it is plain json, parse it and return
                //println!("Handling {:?}", uri);
                let metadata: TokenMetadata = serde_json::from_str(uri)?;
                Ok(metadata)
            }
            _ => match serde_json::from_str(uri) {
                // If it is only the URI without the data format information, try to format it
                // and if it fails, return empty metadata
                Ok(v) => Ok(v),
                Err(_) => Ok(TokenMetadata::default()),
            },
        }
    }

    fn get_metadata_type(uri: &str) -> MetadataType<'_> {
        if uri.starts_with("ipfs://") {
            MetadataType::Ipfs(uri)
        } else if uri.starts_with("http://") || uri.starts_with("https://") {
            MetadataType::Http(uri)
        } else {
            MetadataType::OnChain(uri)
        }
    }
}

pub mod contract {
    use crate::common::starknet_constants::{NAME_SELECTOR, SYMBOL_SELECTOR, ZERO_FELT};
    use crate::common::traits::ToUtf8String;
    use color_eyre::eyre;
    use starknet::core::types::{ContractClass, LegacyContractAbiEntry};
    use starknet::providers::Provider;
    use starknet::{
        core::types::{BlockId, FieldElement, FunctionCall},
        providers::jsonrpc::{HttpTransport, JsonRpcClient},
    };

    pub async fn get_name(
        address: FieldElement,
        block_id: &BlockId,
        rpc: &JsonRpcClient<HttpTransport>,
    ) -> String {
        let request = FunctionCall {
            contract_address: address,
            entry_point_selector: NAME_SELECTOR,
            calldata: vec![],
        };

        let result = rpc.call(request, block_id).await.unwrap_or_default();
        let result = result.get(0).unwrap_or(&ZERO_FELT);

        result.to_utf8_string()
    }

    pub async fn get_symbol(
        address: FieldElement,
        block_id: &BlockId,
        rpc: &JsonRpcClient<HttpTransport>,
    ) -> String {
        let request = FunctionCall {
            contract_address: address,
            entry_point_selector: SYMBOL_SELECTOR,
            calldata: vec![],
        };

        let result = rpc.call(request, block_id).await.unwrap_or_default();
        let result = result.get(0).unwrap_or(&ZERO_FELT);

        result.to_utf8_string()
    }

    /// Returns if given address is EIP721 compliant
    /// This function is required as on Starknet ERC20 and ERC721 tokens have
    /// same "Transfer" event key.
    pub async fn is_erc721(
        address: FieldElement,
        block_id: &BlockId,
        rpc: &JsonRpcClient<HttpTransport>,
    ) -> eyre::Result<bool> {
        let abi = match rpc.get_class_at(block_id, address).await? {
            ContractClass::Sierra(_) => eyre::bail!("sierra not supported yet"),
            ContractClass::Legacy(leg) => leg.abi
        };

        let Some(abi) = abi else {
            return Ok(false);
        };

        // Simplest check we can do is to check "ownerOf" function.
        for abi_entry in abi {
            if let LegacyContractAbiEntry::Function(function_abi_entry) = abi_entry {
                if function_abi_entry.name == "ownerOf" || function_abi_entry.name == "owner_of" {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}
