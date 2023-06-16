#![warn(clippy::all, clippy::pedantic, clippy::style, rust_2018_idioms)]
#![allow(clippy::unreadable_literal)]
mod common;
mod db;
mod events;
mod file_storage;
mod rpc;
use db::postgres::process::ProcessEvent;
use events::{Event, EventBatch};
use sqlx::{Pool, Postgres};
use std::{collections::BinaryHeap, sync::Arc};
use tokio::sync::{mpsc, Mutex, Notify};

use color_eyre::eyre;
use dotenv::dotenv;

use crate::rpc::StarknetRpc;

const EVENT_CHANNEL_BUFFER_SIZE: usize = 10;
const DEFAULT_STARTING_BLOCK: u64 = 1630; // First ERC721 transfer event
const BLOCK_RANGE: u64 = 10;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenv().ok();

    let pool = db::postgres::connect().await?;

    // Drop everything from tables ('course for testing)
    db::postgres::drop_everything(&pool).await?;

    // Both RPC and pool are needed to be instantiated once and used read-only. That's why we're
    // leaking and getting static references out of them
    let rpc: &'static StarknetRpc = Box::leak(Box::new(StarknetRpc::mainnet().unwrap()));
    let pool: &'static Pool<Postgres> = Box::leak(Box::new(pool));

    let (event_tx, event_rx) = mpsc::channel::<EventBatch>(EVENT_CHANNEL_BUFFER_SIZE);

    // Spawn the task that'll read each event_diff message and save them to the database
    tokio::spawn(
        async move { process_event_diffs(event_rx, (&rpc).clone(), (&pool).clone()).await },
    );

    let mut start_block =
        db::postgres::last_synced_block(&pool).await.unwrap_or(DEFAULT_STARTING_BLOCK);

    let MAX_TASK_COUNT: usize = 5;
    let active_task_count = Arc::new(Mutex::new(0_usize));
    let active_task_notify = Arc::new(Notify::new());

    for batch_id in 0_u64.. {
        let task_count = active_task_count.clone();
        let notify = active_task_notify.clone();
        let event_tx = event_tx.clone();

        let from_block = start_block;
        start_block += BLOCK_RANGE;

        tokio::spawn(async move {
            let pool = (&pool).clone();
            let rpc = rpc.clone();
            let handler = events::EventHandler::new(rpc.inner(), pool);

            let mut task_count_before = task_count.lock().await;

            println!("[tx] task count: {}", *task_count_before);

            if *task_count_before >= MAX_TASK_COUNT {
                // Max task count reached, wait for other tasks to finish
                notify.notified().await;
            }

            *task_count_before += 1;
            drop(task_count_before);

            println!("[tx] reading between {} - {}", from_block, from_block + BLOCK_RANGE);
            match rpc.get_transfer_events(from_block, BLOCK_RANGE).await {
                Ok(transfer_events) => {
                    let block_diff = handler.read_events(batch_id, &transfer_events).await?;
                    event_tx.send(block_diff).await?;
                    println!("[tx] sent batch #{batch_id} to the channel");
                }
                Err(e) => {
                    println!("{:?}", e);
                }
            }

            println!("[tx] failure");

            let mut task_count_after = task_count.lock().await;
            *task_count_after -= 1;

            if *task_count_after < MAX_TASK_COUNT {
                // Current task is done, notify a new one
                notify.notify_one();
            }

            eyre::Result::<(), eyre::ErrReport>::Ok(())
        });
    }

    Ok(())
}

async fn process_event_diffs(
    mut event_rx: mpsc::Receiver<EventBatch>,
    rpc: &'static StarknetRpc,
    pool: &Pool<Postgres>,
) -> eyre::Result<()> {
    let mut latest_batch_id: u64 = 0;
    let mut batch_id_heap = BinaryHeap::<u64>::with_capacity(EVENT_CHANNEL_BUFFER_SIZE);
    let mut batches = Vec::<EventBatch>::with_capacity(EVENT_CHANNEL_BUFFER_SIZE);

    while let Some(batch) = event_rx.recv().await {
        println!("[rx] got a new batch (#{})", batch.batch_id());
        batch_id_heap.push(batch.batch_id());
        batches.push(batch);

        while batch_id_heap.peek() == Some(latest_batch_id + 1).as_ref() {
            // Process and pop pending block
            let pending_block_idx = batches
                .iter()
                .position(|item| item.batch_id() == batch_id_heap.pop().unwrap())
                .unwrap();
            let pending_block = batches.remove(pending_block_idx);

            let mut transaction = pool.begin().await?;

            for event in pending_block.into_events() {
                match event {
                    Event::Erc721Transfer(event) => {
                        event.process(rpc.inner(), &mut transaction).await?
                    }
                    Event::Erc1155TransferSingle(event) => {
                        event.process(rpc.inner(), &mut transaction).await?
                    }
                    Event::Erc1155TransferBatch(event) => {
                        event.process(rpc.inner(), &mut transaction).await?
                    }
                }
            }
            // TODO: Proces the block diff

            latest_batch_id += 1;
            transaction.commit().await?;
        }
    }

    Ok(())
}
