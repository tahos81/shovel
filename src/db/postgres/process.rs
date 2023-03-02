use crate::events::EventHandler;
use async_trait::async_trait;
use color_eyre::eyre::Result;

#[async_trait]
pub trait ProcessEvent {
    async fn process(&self, handler: &mut EventHandler<'_, '_>) -> Result<()>;
}
