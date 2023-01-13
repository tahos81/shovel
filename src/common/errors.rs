// Custom errors used through the project
use std::env::VarError;
use thiserror::Error;
use url::ParseError;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Configure .env file")]
    MissingField(#[from] VarError),
    #[error("Invalid rpc url")]
    InvalidRpcUrl(#[from] ParseError),
    #[error("Something is wrong with your connection string")]
    InvalidConnectionString(#[from] mongodb::error::Error),
}

#[derive(Debug, Error)]
pub enum RpcError {}
