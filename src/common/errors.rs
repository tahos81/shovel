// Custom errors used through the projects
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RpcError {
    #[error("Could not setup rpc")]
    SetupError,
}
