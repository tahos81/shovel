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
use tokio::sync::{mpsc, Semaphore};

use color_eyre::eyre;
use dotenv::dotenv;

use crate::rpc::StarknetRpc;

const EVENT_CHANNEL_BUFFER_SIZE: usize = 10;
const DEFAULT_STARTING_BLOCK: u64 = 1630; // First ERC721 transfer event
const BLOCK_RANGE: u64 = 10;
const MAX_TASK_COUNT: usize = 3;

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

    let mut start_block =
        db::postgres::last_synced_block(&pool).await.unwrap_or(DEFAULT_STARTING_BLOCK);

    let task_permit = Arc::new(Semaphore::new(MAX_TASK_COUNT));

    for batch_id in 0_u64.. {
        let task_permit = task_permit.clone();
        let event_tx = event_tx.clone();

        let from_block = start_block;
        start_block += BLOCK_RANGE;

        tokio::spawn(async move {
            let _permit = task_permit.acquire().await.expect("Semaphore acquire");
            println!(
                "[tx-{}] reading between {}-{}",
                batch_id,
                from_block,
                from_block + BLOCK_RANGE
            );

            let pool = (&pool).clone();
            let rpc = rpc.clone();
            let handler = events::EventHandler::new(rpc.inner(), pool);
            match rpc.get_transfer_events(from_block, BLOCK_RANGE).await {
                Ok(transfer_events) => {
                    let block_diff = handler.read_events(batch_id, &transfer_events).await?;
                    event_tx.send(block_diff).await?;
                    println!("[tx-{}] sent to the channel", batch_id);
                }
                Err(e) => {
                    println!("[tx-{}] failure: {:?}", batch_id, e);
                }
            }
            eyre::Result::<(), eyre::ErrReport>::Ok(())
        });
    }

    println!("Exiting for some reason");
    Ok(())
}

/// Reads from the `EventBatch` receiver and writes the events to the database
///
/// # Notes
/// This function runs in a seperate tokio task where it processes events coming
/// from multiple event handler events.
///
/// In order for this mechanism to work properly it should work faster than the
/// event handler threads. Considering most of the performance bottleneck is
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
        println!("[rx] got a new batch #{}, latest: #{}", batch.batch_id(), latest_batch_id);
        batch_id_heap.push(Reverse(batch.batch_id()));
        batches.push(batch);

        while batch_id_heap.peek() == Some(Reverse(latest_batch_id)).as_ref() {
            // Process and pop pending block
            let search_id = batch_id_heap.pop().unwrap();
            println!("[rx] Searching for id #{:?}", search_id.0);
            let pending_batch_idx =
                batches.iter().position(|item| item.batch_id() == search_id.0).unwrap();
            let pending_batch = batches.remove(pending_batch_idx);

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

    println!("[rx] exit??");

    Ok(())
}
