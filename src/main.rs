#![warn(clippy::all, clippy::pedantic, clippy::style, rust_2018_idioms)]
#![allow(clippy::unreadable_literal)]
mod common;
mod db;
mod events;
mod file_storage;
mod rpc;

use color_eyre::eyre::Result;
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let rpc = rpc::connect()?;

    let conn_str = std::env::var("DATABASE_URL").expect("Env var DATABASE_URL is required");
    let pool = sqlx::PgPool::connect(&conn_str).await?;

    // Drop everythin from tables
    db::postgres::drop_everything(&pool).await?;

    //first transfer event is in 1630
    let mut start_block = db::postgres::last_synced_block(&pool).await?;
    let range = 10;

    while start_block < 16000 {
        println!("getting events between block {} and {}", start_block, start_block + range);
        let transfer_events = rpc::get_transfer_events::run(start_block, range, &rpc).await?;
        println!("got {} events in total", transfer_events.len());

        let mut transaction = pool.begin().await?;
        let mut event_handler = events::EventHandler::new(&rpc, &mut transaction);

        for transfer_event in &transfer_events {
            event_handler.handle(transfer_event).await?;
        }
        db::postgres::update_last_synced_block(start_block, &mut transaction).await?;
        transaction.commit().await?;

        println!("events handled");

        start_block += range;
    }

    Ok(())
}