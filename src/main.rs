#![warn(clippy::all, clippy::pedantic, clippy::style, rust_2018_idioms)]
#![allow(clippy::unreadable_literal)]
mod common;
mod db;
mod event_handlers;
mod rpc;

use clap::Parser;
use clap_verbosity_flag::Verbosity;
use color_eyre::eyre::Result;
use dotenv::dotenv;
use mongodb::error::UNKNOWN_TRANSACTION_COMMIT_RESULT;

#[derive(Debug, Parser)]
struct Cli {
    #[clap(flatten)]
    pub verbose: Verbosity,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let cli = Cli::parse();

    env_logger::Builder::new().filter_level(cli.verbose.log_level_filter()).init();

    let rpc = rpc::connect()?;
    let (db, mut session) = db::connect().await?;

    db::drop_collections(&db).await?;

    //first transfer event is in 1630
    let mut start_block = db::last_synced_block(&db, &mut session).await;
    let range = 30;

    while start_block < 16000 {
        session.start_transaction(None).await?;

        log::info!("getting events between block {} and {}", start_block, start_block + range);
        let transfer_events = rpc::get_transfer_events::run(start_block, range, &rpc).await?;
        log::info!("got {} events in total", transfer_events.len());

        event_handlers::handle_transfer_events(transfer_events, &rpc, &db, &mut session).await?;

        start_block += range;
        db::update_last_synced_block(&db, start_block, &mut session).await?;

        loop {
            let result = session.commit_transaction().await;
            if let Err(ref error) = result {
                if error.contains_label(UNKNOWN_TRANSACTION_COMMIT_RESULT) {
                    continue;
                }
            }
            break result?;
        }
        log::info!("committed transaction");
    }

    Ok(())
}
