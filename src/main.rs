#![warn(clippy::all, clippy::pedantic, clippy::style, rust_2018_idioms)]
#![allow(clippy::unreadable_literal)]
mod common;
mod db;
mod events;
mod file_storage;
mod rpc;
use sqlx::{Pool, Postgres};
use std::collections::BinaryHeap;
use tokio::sync::mpsc::{self, Receiver};

use color_eyre::eyre::Result;
use dotenv::dotenv;

use crate::{events::BlockDiff, rpc::StarknetRpc};

const EVENT_CHANNEL_BUFFER_SIZE: usize = 10;
const DEFAULT_STARTING_BLOCK: u64 = 1630; // First ERC721 transfer event
const BLOCK_RANGE: u64 = 10;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let pool = db::postgres::connect().await?;

    // Drop everything from tables ('course for testing)
    db::postgres::drop_everything(&pool).await?;

    let start_block =
        db::postgres::last_synced_block(&pool).await.unwrap_or(DEFAULT_STARTING_BLOCK);

    // Both RPC and pool are needed to be instantiated once and used read-only. That's why we're
    // leaking it and get static references out of them
    let rpc: &'static StarknetRpc = Box::leak(Box::new(StarknetRpc::mainnet().unwrap()));
    let pool: &'static Pool<Postgres> = Box::leak(Box::new(pool));

    let (event_tx, event_rx) = mpsc::channel::<BlockDiff>(EVENT_CHANNEL_BUFFER_SIZE);

    // Spawn the task that'll read each event_diff message and save them to the database
    tokio::spawn(async move { process_event_diffs(event_rx, &pool.clone()).await });

    for batch_id in 0_u64.. {
        tokio::spawn(async move {
            let handler = events::EventHandler::new(rpc.clone().inner(), &pool.clone());

            if let Ok(transfer_events) = rpc.get_transfer_events(start_block, BLOCK_RANGE).await {
                let event_tx = event_tx.clone();
                let block_diff = handler.diff_from_events(batch_id, &transfer_events).await?;
                event_tx.send(block_diff).await;
            }
        });
    }

    Ok(())
}

async fn process_event_diffs(
    mut event_rx: Receiver<BlockDiff>,
    pool: &Pool<Postgres>,
) -> Result<()> {
    let mut latest_batch_id: u64 = 0;
    let mut block_num_heap = BinaryHeap::<u64>::with_capacity(EVENT_CHANNEL_BUFFER_SIZE);
    let mut blocks = Vec::<BlockDiff>::with_capacity(EVENT_CHANNEL_BUFFER_SIZE);

    for block in event_rx.recv().await {
        block_num_heap.push(block.batch_id());
        blocks.push(block);

        while block_num_heap.peek() == Some(latest_batch_id + 1).as_ref() {
            // Process and pop pending block
            let pending_block_idx = blocks
                .iter()
                .position(|item| item.batch_id() == block_num_heap.pop().unwrap())
                .unwrap();
            let pending_block = blocks.remove(pending_block_idx);

            let transaction = pool.begin().await?;

            // TODO: Proces the block diff

            latest_batch_id += 1;
            transaction.commit().await?;
        }
    }

    Ok(())
}
