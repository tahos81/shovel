pub mod get_transfer_events;
pub mod metadata;

use crate::common::errors::ConfigError;
use color_eyre::eyre::Result;
use reqwest::Url;
use starknet::providers::jsonrpc::{HttpTransport, JsonRpcClient};
use std::env;

pub fn connect() -> Result<JsonRpcClient<HttpTransport>, ConfigError> {
    let rpc_url = env::var("STARKNET_MAINNET_RPC")?;
    let parsed_url = Url::parse(&rpc_url)?;
    Ok(JsonRpcClient::new(HttpTransport::new(parsed_url)))
}
