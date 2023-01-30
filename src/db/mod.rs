pub mod collection;
pub mod document;

use crate::common::errors::ConfigError;
use document::{ContractMetadata, Erc1155Balance, Erc1155Metadata, Erc721};
use mongodb::{
    bson::doc,
    options::{ClientOptions, UpdateOptions},
    Client, ClientSession, Collection, Database,
};
use std::env;

pub async fn connect() -> Result<(Database, ClientSession), ConfigError> {
    let client_url_with_options = env::var("CONNECTION_STRING_WITH_OPTIONS")?;
    let client_options = ClientOptions::parse(client_url_with_options).await?;

    let client = Client::with_options(client_options)?;
    let session = client.start_session(None).await?;
    Ok((client.database("shovel"), session))
}

pub async fn last_synced_block(db: &Database, session: &mut ClientSession) -> Option<u64> {
    let commons: Collection<u64> = db.collection("common");

    commons.find_one_with_session(None, None, session).await.unwrap_or(None)
}

pub async fn update_last_synced_block(
    db: &Database,
    last_sync: u64,
    session: &mut ClientSession,
) -> color_eyre::eyre::Result<()> {
    let commons: Collection<u64> = db.collection("common");

    let update = doc! {
        "$set": {"last_sync": last_sync as i64}
    };

    let options = UpdateOptions::builder().upsert(true).build();

    commons.update_one_with_session(doc! {}, update, options, session).await?;
    Ok(())
}

pub async fn drop_collections(db: &Database) -> color_eyre::eyre::Result<()> {
    // Drop all collections for test purposes
    db.collection::<Erc721>("erc721_tokens").delete_many(doc! {}, None).await?;
    db.collection::<Erc1155Balance>("erc1155_tokens").delete_many(doc! {}, None).await?;
    db.collection::<Erc1155Metadata>("erc1155_metadata").delete_many(doc! {}, None).await?;
    db.collection::<ContractMetadata>("contract_metadata").delete_many(doc! {}, None).await?;
    println!("dropped all collections");

    Ok(())
}
