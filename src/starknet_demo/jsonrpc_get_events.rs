use starknet::providers::jsonrpc::{
    models::BlockId, models::EventFilter, HttpTransport, JsonRpcClient,
};

use reqwest::Url;

pub async fn run() {
    let rpc = JsonRpcClient::new(HttpTransport::new(
        Url::parse("https://starknet-goerli.rpc.zklend.com/").unwrap(),
    ));

    let filter: EventFilter = EventFilter {
        from_block: Some(BlockId::Number(234500)),
        to_block: Some(BlockId::Number(234501)),
        address: None,
        keys: None,
    };

    let events = rpc.get_events(filter, None, 64).await.unwrap();

    dbg!(events);
}
