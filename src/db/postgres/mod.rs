pub mod process;

// TODO: Pack following functions into a trait that all databases can implement

use color_eyre::eyre::{eyre, Result};
use sqlx::{query, Pool, Postgres};

pub async fn drop_everything(pool: &Pool<Postgres>) -> Result<()> {
    let mut transaction = pool.begin().await?;

    query!("DELETE FROM contract_metadata").execute(&mut transaction).await?;
    query!("DELETE FROM token_metadata").execute(&mut transaction).await?;
    query!("DELETE FROM erc721_data").execute(&mut transaction).await?;
    query!("DELETE FROM erc721_owners").execute(&mut transaction).await?;
    query!("DELETE FROM erc1155_balances").execute(&mut transaction).await?;

    Ok(())
}

pub async fn last_synced_block(pool: &Pool<Postgres>) -> Result<u64> {
    let last_synced_block =
        query!("SELECT last_synced_block FROM sync_data").fetch_one(pool).await?.last_synced_block;

    last_synced_block
        .ok_or(eyre!("last_synced_block is null"))
        .map(|block_number| block_number as u64)
}

pub async fn update_last_synced_block(block_number: u64, pool: &Pool<Postgres>) -> Result<()> {
    query!(
        "UPDATE sync_data SET last_synced_block = $1",
        i64::try_from(block_number).expect("block_number parse fail")
    )
    .execute(pool)
    .await?;

    Ok(())
}
