use crate::events::context::Event;
use async_trait::async_trait;
use color_eyre::eyre::Result;

#[async_trait]
pub trait ProcessEvent {
    async fn process(
        &mut self,
        ctx: &Event<'_, '_>,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()>;
}
