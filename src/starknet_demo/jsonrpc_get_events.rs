use starknet::core::types::FieldElement;
use starknet::providers::jsonrpc::models::ContractAbiEntry::Function;
use starknet::providers::jsonrpc::models::{EmittedEvent, EventsPage};
use starknet::providers::jsonrpc::{
    models::BlockId, models::EventFilter, HttpTransport, JsonRpcClient,
};

use dotenv::dotenv;
use reqwest::Url;
use std::collections::HashSet;
use std::env;

/// felt!("0x99cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9");
const TRANSFER_EVENT_KEY: FieldElement = FieldElement::from_mont([
    10370298062762752593,
    7288672513944573579,
    6148261015514870755,
    242125613396778233,
]);

/// felt!("0x182d859c0807ba9db63baf8b9d9fdbfeb885d820be6e206b9dab626d995c433");
const TRANSFER_SINGLE_EVENT_KEY: FieldElement = FieldElement::from_mont([
    1986363494579022220,
    17146673375846491535,
    6125027481420860397,
    307829215948623223,
]);

/// felt!("0x2563683c757f3abe19c4b7237e2285d8993417ddffe0b54a19eb212ea574b08");
const TRANSFER_BATCH_EVENT_KEY: FieldElement = FieldElement::from_mont([
    14114721770411318090,
    10106114908748783105,
    12894248477188639378,
    518981439849896716,
]);

pub async fn run() -> Vec<EmittedEvent> {
    dotenv().ok();

    let mut ret: Vec<EmittedEvent> = Vec::new();

    //TODO: Replace it with database
    let mut whitelist: HashSet<FieldElement> = HashSet::new();
    let mut blacklist: HashSet<FieldElement> = HashSet::new();

    let rpc_url = env::var("STARKNET_MAINNET_RPC").expect("configure your .env file");
    let rpc = JsonRpcClient::new(HttpTransport::new(Url::parse(&rpc_url).unwrap()));

    let keys: Vec<FieldElement> = Vec::from([
        TRANSFER_EVENT_KEY,
        TRANSFER_SINGLE_EVENT_KEY,
        TRANSFER_BATCH_EVENT_KEY,
    ]);

    let starting_block = 14791;
    let block_range = 0;

    let filter: EventFilter = EventFilter {
        from_block: Some(BlockId::Number(starting_block)),
        to_block: Some(BlockId::Number(starting_block + block_range)),
        address: None,
        keys: Some(keys),
    };

    let mut continuation_token: Option<String> = None;
    let chunk_size: u64 = 1024;

    let mut events: EventsPage;

    loop {
        events = rpc
            //HOW CAN I REFRAIN FROM CLONING FILTER
            .get_events(filter.clone(), continuation_token, chunk_size)
            .await
            .unwrap();

        for event in events.events {
            if event.keys.contains(&TRANSFER_EVENT_KEY) {
                //possible ERC721
                let address = event.from_address;
                if whitelist.contains(&address) {
                    //dbg!(event);
                    continue;
                } else if blacklist.contains(&address) {
                    continue;
                } else {
                    if is_erc721(address, BlockId::Number(event.block_number), &rpc).await {
                        whitelist.insert(address);
                        ret.push(event);
                        //dbg!(event);
                    } else {
                        blacklist.insert(address);
                    }
                }
            } else {
                //definitely ERC1155
                //dbg!(event);
            }
        }

        continuation_token = events.continuation_token;

        if continuation_token.is_none() {
            break;
        }
    }

    ret
}

async fn is_erc721(
    address: FieldElement,
    block_id: BlockId,
    rpc: &JsonRpcClient<HttpTransport>,
) -> bool {
    let abi = rpc
        .get_class_at(&block_id, address)
        .await
        .unwrap()
        .abi
        .unwrap();

    for abi_entry in abi {
        match abi_entry {
            Function(function_abi_entry) => {
                if function_abi_entry.name == "ownerOf" || function_abi_entry.name == "owner_of" {
                    return true;
                }
            }
            _ => {}
        }
    }

    false
}
