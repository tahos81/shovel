pub mod collection_interface;
pub mod document;

use crate::common::errors::ConfigError;
use mongodb::{options::ClientOptions, Client, Database};
use std::env;

pub async fn connect() -> Result<Database, ConfigError> {
    let client_url_with_options = env::var("CONNECTION_STRING_WITH_OPTIONS")?;
    let client_options = ClientOptions::parse(client_url_with_options).await?;

    let client = Client::with_options(client_options)?;
    Ok(client.database("shovel"))
}
