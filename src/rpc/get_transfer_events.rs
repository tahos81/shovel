use crate::common::starknet_constants::{
    TRANSFER_BATCH_EVENT_KEY, TRANSFER_EVENT_KEY, TRANSFER_SINGLE_EVENT_KEY,
};
use color_eyre::eyre::Result;
use starknet::core::types::FieldElement;
use starknet::providers::jsonrpc::{
    models::{BlockId, EmittedEvent, EventFilter, EventsPage},
    {HttpTransport, JsonRpcClient},
};

pub async fn run(
    start_block: u64,
    range: u64,
    rpc: &JsonRpcClient<HttpTransport>,
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
            rpc.get_events(transfer_filter.clone(), continuation_token, chunk_size).await?;

        println!("got {} events", get_events_resp.events.len());
        events.append(&mut get_events_resp.events);
        continuation_token = get_events_resp.continuation_token;

        if continuation_token.is_none() {
            break Ok(events);
        }
    }
}
