pub mod metadata;

use crate::common::{
    errors::ConfigError,
    starknet_constants::{TRANSFER_BATCH_EVENT_KEY, TRANSFER_EVENT_KEY, TRANSFER_SINGLE_EVENT_KEY},
};
use color_eyre::eyre::Result;
use reqwest::Url;
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{
        models::{BlockId, EmittedEvent, EventFilter, EventsPage},
        HttpTransport, JsonRpcClient,
    },
};
use std::env;

pub struct StarknetRpc(JsonRpcClient<HttpTransport>);

impl StarknetRpc {
    pub fn mainnet() -> Result<Self, ConfigError> {
        let rpc_url = env::var("STARKNET_MAINNET_RPC")?;
        let parsed_url = Url::parse(&rpc_url)?;
        Ok(Self(JsonRpcClient::new(HttpTransport::new(parsed_url))))
    }

    pub fn inner(&self) -> &JsonRpcClient<HttpTransport> {
        &self.0
    }

    pub async fn get_transfer_events(
        &self,
        start_block: u64,
        range: u64,
    ) -> Result<Vec<EmittedEvent>> {
        let keys: Vec<FieldElement> =
            Vec::from([TRANSFER_EVENT_KEY, TRANSFER_SINGLE_EVENT_KEY, TRANSFER_BATCH_EVENT_KEY]);

        let transfer_filter = EventFilter {
            from_block: Some(BlockId::Number(start_block)),
            to_block: Some(BlockId::Number(start_block + range)),
            address: None,
            keys: Some(keys),
        };

        let mut continuation_token: Option<String> = None;
        let chunk_size: u64 = 1024;

        let mut get_events_resp: EventsPage;
        let mut events: Vec<EmittedEvent> = Vec::new();

        loop {
            get_events_resp =
                self.0.get_events(transfer_filter.clone(), continuation_token, chunk_size).await?;

            println!("[rpc] got {} events", get_events_resp.events.len());
            events.append(&mut get_events_resp.events);
            continuation_token = get_events_resp.continuation_token;

            if continuation_token.is_none() {
                break Ok(events);
            }
        }
    }
}