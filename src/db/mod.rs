pub mod collection;
pub mod document;

use crate::common::errors::ConfigError;
use mongodb::{options::ClientOptions, Client, ClientSession, Database};
use std::env;

pub async fn connect() -> Result<(Database, ClientSession), ConfigError> {
    let client_url_with_options = env::var("CONNECTION_STRING_WITH_OPTIONS")?;
    let client_options = ClientOptions::parse(client_url_with_options).await?;

    let client = Client::with_options(client_options)?;
    let session = client.start_session(None).await?;
    Ok((client.database("shovel"), session))
}
