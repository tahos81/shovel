#![warn(clippy::all, clippy::pedantic, clippy::style, rust_2018_idioms)]
#![allow(clippy::unreadable_literal, clippy::module_name_repetitions, clippy::too_many_lines)]
mod common;
mod db;
mod events;
mod file_storage;
mod rpc;
use db::postgres::process::ProcessEvent;
use events::EventBatch;
use sqlx::{Pool, Postgres};
use starknet::providers::Provider;
use std::{cmp::Reverse, collections::BinaryHeap, sync::Arc};
use tokio::sync::{mpsc, Mutex};

use color_eyre::eyre;
use dotenv::dotenv;

use crate::{db::postgres::update_last_synced_block, rpc::StarknetRpc};

// Default starting block 1630 is around the first ERC721 Transfer event
const DEFAULT_STARTING_BLOCK: u64 = 1630;
const BLOCK_RANGE: u64 = 10;
// Number of concurrent tasks
const MAX_TASK_COUNT: usize = 10;
// Initial size of the Vec that is going to hold EventCache's accumulated from
// different tasks until they are executed
const EVENT_CHANNEL_BUFFER_SIZE: usize = MAX_TASK_COUNT * 2;
const LAST_SYNC_UPDATE_INTERVAL: u64 = 5;

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

    let (event_tx, event_rx) = mpsc::channel::<Box<EventBatch>>(EVENT_CHANNEL_BUFFER_SIZE);

    // Spawn the writer thread
    tokio::spawn(async move {
        let rpc = <&StarknetRpc>::clone(&rpc);
        let pool = <&Pool<Postgres>>::clone(&pool);
        write_events(event_rx, rpc, pool).await.unwrap();
        println!("Writer thread closed");
    });

    let start_block = db::postgres::last_synced_block(pool).await.unwrap_or(DEFAULT_STARTING_BLOCK);
    let batch_id = Arc::new(Mutex::new(0_u64));
    let mut reader_threads = Vec::new();

    for thread_id in 0..MAX_TASK_COUNT {
        let event_tx = event_tx.clone();
        let batch_id = batch_id.clone();

        let thread_handle = tokio::spawn(async move {
            loop {
                // Acquire and increment batch id
                let current_batch_id = {
                    let mut lock = batch_id.lock().await;
                    let latest_value = *lock;
                    *lock += 1;
                    latest_value
                };

                let from_block = start_block + BLOCK_RANGE * current_batch_id;
                println!(
                    "[tx-{}] reading between {}-{}, batch id: {}",
                    thread_id,
                    from_block,
                    from_block + BLOCK_RANGE,
                    current_batch_id,
                );
                let pool = <&Pool<Postgres>>::clone(&pool);
                let rpc = <&StarknetRpc>::clone(&rpc);
                let handler = events::EventHandler::new(rpc.inner(), pool);

                let empty_batch = EventBatch::new(current_batch_id, from_block, vec![]);
                let whitelist = db::postgres::whitelist(&pool).await;
                match rpc.get_transfer_events(from_block, BLOCK_RANGE).await {
                    Ok(transfer_events) => {
                        let batch = match handler
                            .read_events(
                                current_batch_id,
                                from_block,
                                &transfer_events,
                                events::EventFilter::Whitelist(&whitelist),
                            )
                            .await
                        {
                            Ok(batch) => batch,
                            Err(_) => empty_batch,
                        };
                        println!("[tx-{}], got {} events", thread_id, batch.events().len());

                        event_tx.send(Box::new(batch)).await.unwrap();
                        println!("[tx-{thread_id}] sent to the channel");
                    }
                    Err(e) => {
                        eprintln!("[tx-{thread_id}] failure: {e}");
                        event_tx.send(Box::new(empty_batch)).await.expect("Send batch");
                    }
                }
            }
        });

        reader_threads.push(thread_handle);
    }

    for thread in reader_threads {
        thread.await.unwrap();
    }

    Ok(())
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
/// the least amount of blockchain reads, ideally none.
///
/// # Errors
/// This function returns `eyre::ErrReport` if there's problem with starting
/// or commiting the Transaction
///
/// # Panics
/// This function panics if it can't find the `pending_batch` in the batches,
/// which should be something impossible.
async fn write_events(
    mut event_rx: mpsc::Receiver<Box<EventBatch>>,
    rpc: &'static StarknetRpc,
    pool: &Pool<Postgres>,
) -> eyre::Result<()> {
    let mut latest_batch_id: u64 = 0;
    let mut batch_id_heap = BinaryHeap::<Reverse<u64>>::with_capacity(EVENT_CHANNEL_BUFFER_SIZE);
    let mut batches = Vec::<Box<EventBatch>>::with_capacity(EVENT_CHANNEL_BUFFER_SIZE);

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
            let from_block_number = pending_batch.start_block_number();

            println!("[rx] Writing id #{:?} to DB", search_id.0);
            let mut transaction = pool.begin().await?;

            for event in pending_batch.into_events() {
                if let Err(e) = event.process(rpc.inner(), &mut transaction).await {
                    eprintln!("[rx] error while writing, {e}");
                }
            }

            latest_batch_id += 1;
            if latest_batch_id % LAST_SYNC_UPDATE_INTERVAL == 0 {
                update_last_synced_block(from_block_number, &mut transaction).await.unwrap();
            }

            transaction.commit().await?;
        }
    }

    Ok(())
}
