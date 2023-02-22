use async_trait::async_trait;
use sqlx::{Pool, Postgres};

#[async_trait]
pub trait ProcessEvent {
    async fn process(&mut self, pool: &mut Pool<Postgres>);
}
