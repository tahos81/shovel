// Custom errors used through the project
use std::env::VarError;
use thiserror::Error;
use url::ParseError;

#[derive(Debug, Error)]
pub enum RpcError {
    #[error("Configure .env file")]
    MissingField(#[from] VarError),
    #[error("Invalid rpc url")]
    InvalidUrl(#[from] ParseError),
}
