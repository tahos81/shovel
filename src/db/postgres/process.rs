use async_trait::async_trait;
use color_eyre::eyre::Result;
use sqlx::Postgres;
use starknet::providers::jsonrpc::{HttpTransport, JsonRpcClient};

#[async_trait]
pub trait ProcessEvent {
    async fn process(
        &self,
        rpc: &'static JsonRpcClient<HttpTransport>,
        transaction: &mut sqlx::Transaction<'_, Postgres>,
    ) -> Result<()>;
}
