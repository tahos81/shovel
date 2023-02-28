use async_trait::async_trait;
use color_eyre::eyre::Result;
use starknet::providers::jsonrpc::{JsonRpcClient, HttpTransport};

#[async_trait]
pub trait ProcessEvent {
    async fn process(
        &mut self,
        rpc: &JsonRpcClient<HttpTransport>,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()>;
}
