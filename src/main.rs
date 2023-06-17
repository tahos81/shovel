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
use std::{cmp::Reverse, collections::BinaryHeap, sync::Arc};
use tokio::sync::{mpsc, Mutex};

use color_eyre::eyre;
use dotenv::dotenv;

use crate::rpc::StarknetRpc;

// Initial size of the Vec that is going to hold EventCache's accumulated from
// different tasks until they are executed
const EVENT_CHANNEL_BUFFER_SIZE: usize = 20;
// Default starting block 1630 is around the first ERC721 Transfer event
const DEFAULT_STARTING_BLOCK: u64 = 1630; 
const BLOCK_RANGE: u64 = 10;
// Number of concurrent tasks
const MAX_TASK_COUNT: u64 = 5;

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
    tokio::spawn(async move { write_events(event_rx, (&rpc).clone(), (&pool).clone()).await });

    let start_block =
        db::postgres::last_synced_block(&pool).await.unwrap_or(DEFAULT_STARTING_BLOCK);
    let batch_id = Arc::new(Mutex::new(0_u64));
    let active_task_count = Arc::new(Mutex::new(0_u64));

    loop {
        let batch_id = batch_id.clone();

        // Before spawning a task check the active task count and make sure
        // that we don't have too many tasks at once
        let task_count_check = active_task_count.clone();
        let task_count_check = task_count_check.lock().await;
        if *task_count_check >= MAX_TASK_COUNT {
            continue;
        }

        let task_count = active_task_count.clone();
        let event_tx = event_tx.clone();

        tokio::spawn(async move {
            // Before each task, lock the task count and learn about how many
            // tasks are there. In order for other tasks to have this information
            // unlock it (by dropping) immediately.
            let mut task_count_before = task_count.lock().await;
            let current_task_count = *task_count_before;

            // If there are tasks more than or equal to MAX_TASK_COUNT, wait to
            // get notified.
            if current_task_count >= MAX_TASK_COUNT {
                // Max task count reached, wait for other tasks to finish
                // println!("[tx] task limit reached, waiting, ({})", current_task_count);
                // drop(task_count_before);
                // notify.notified().await;
                return Ok(());
                // println!("[tx] notified");
            } else {
                // Lock the task count briefly and increment it
                // let mut task_count_before = task_count.lock().await;
                // println!("[tx] spawned with task_count {}", *task_count_before);
                *task_count_before += 1;
                drop(task_count_before);
            }

            // Since we started running the task, increment batch id as well
            let mut batch_id = batch_id.lock().await;
            let current_batch_id = *batch_id;
            *batch_id += 1;
            drop(batch_id); // Again, drop it so that we don't block anything

            let from_block = start_block + BLOCK_RANGE * current_batch_id;
            println!(
                "[tx] reading between {}-{}, batch id: {}",
                from_block,
                from_block + BLOCK_RANGE,
                current_batch_id,
            );
            let pool = (&pool).clone();
            let rpc = rpc.clone();
            let handler = events::EventHandler::new(rpc.inner(), pool);
            match rpc.get_transfer_events(from_block, BLOCK_RANGE).await {
                Ok(transfer_events) => {
                    let batch = handler.read_events(current_batch_id, &transfer_events).await?;
                    event_tx.send(batch).await?;
                    println!("[tx-{}] sent to the channel", current_batch_id);
                }
                Err(e) => {
                    println!("[tx-{}] failure: {:?}", current_batch_id, e);
                    // When there's an error we don't want to brick rx task,
                    // so send an empty message. Ideally, we shouldn't error unless
                    // something critical happens
                    // TODO: Increase error tolerance of `EventHandler::read_events`
                    event_tx.send(EventBatch::new(current_batch_id, Vec::new())).await?;
                }
            }

            // Lock the task count again and decrement it
            let mut task_count_after = task_count.lock().await;
            *task_count_after -= 1;

            eyre::Result::<(), eyre::ErrReport>::Ok(())
        });
    }
}

/// Reads from the `EventBatch` receiver and writes the events to the database
///
/// # Notes
/// This function runs in a seperate tokio task where it processes events coming
/// from multiple event handler events.
///
/// In order for this mechanism to work properly it should work faster than the
/// event handler tasks. Considering most of the performance bottleneck is
/// blockchain reads, this function and `ProcessEvent` implementations should do
/// the least amount of blockchain reads
///
/// # Errors
/// This function returns `eyre::ErrReport` if there's problem with starting
/// or commiting the Transaction
///
/// # Panics
/// This function panics if it can't find the pending_batch in the batches,
/// which should be something impossible.
async fn write_events(
    mut event_rx: mpsc::Receiver<EventBatch>,
    rpc: &'static StarknetRpc,
    pool: &Pool<Postgres>,
) -> eyre::Result<()> {
    let mut latest_batch_id: u64 = 0;
    let mut batch_id_heap = BinaryHeap::<Reverse<u64>>::with_capacity(EVENT_CHANNEL_BUFFER_SIZE);
    let mut batches = Vec::<EventBatch>::with_capacity(EVENT_CHANNEL_BUFFER_SIZE);

    while let Some(batch) = event_rx.recv().await {
        println!("[rx] got a new batch {}, latest: {}", batch.batch_id(), latest_batch_id);
        batch_id_heap.push(Reverse(batch.batch_id()));
        batches.push(batch);

        while batch_id_heap.peek() == Some(Reverse(latest_batch_id)).as_ref() {
            // Process and pop pending block
            let search_id = batch_id_heap.pop().unwrap();
            let pending_batch_idx =
                batches.iter().position(|item| item.batch_id() == search_id.0).unwrap();
            let pending_batch = batches.remove(pending_batch_idx);

            println!("[rx] Writing id #{:?} to DB", search_id.0);
            let mut transaction = pool.begin().await?;

            for event in pending_batch.into_events() {
                match event {
                    Event::Erc721Transfer(event) => {
                        println!("[rx] processing ERC721 Transfer event");
                        event.process(rpc.inner(), &mut transaction).await?
                    }
                    Event::Erc1155TransferSingle(event) => {
                        println!("[rx] processing ERC1155 TransferSingle event");
                        event.process(rpc.inner(), &mut transaction).await?
                    }
                    Event::Erc1155TransferBatch(event) => {
                        println!("[rx] processing ERC1155 TransferBatch event");
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
