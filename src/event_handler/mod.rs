use crate::rpc;
use crate::rpc::starknet_constants::*;
use starknet::providers::jsonrpc::models::{BlockId, EmittedEvent};
use starknet::providers::jsonrpc::{HttpTransport, JsonRpcClient};

pub async fn handle_transfer_events(
    transfer_events: Vec<EmittedEvent>,
    rpc: &JsonRpcClient<HttpTransport>,
) {
    for transfer_event in transfer_events {
        if transfer_event.keys.contains(&TRANSFER_EVENT_KEY) {
            //possible ERC721
            let contract_address = transfer_event.from_address;
            let block_id = BlockId::Number(transfer_event.block_number);
            if rpc::is_erc721(contract_address, block_id, &rpc).await {
                handle_erc721_event(transfer_event);
            }
        } else {
            //definitely ERC1155
            handle_erc1155_event(transfer_event);
        }
    }
}

fn handle_erc721_event(erc721_event: EmittedEvent) {
    if erc721_event.data[0] == ZERO_ADDRESS {
        handle_erc721_mint(erc721_event);
    } else if erc721_event.data[1] == ZERO_ADDRESS {
        handle_erc721_burn(erc721_event);
    } else {
        handle_erc721_transfer(erc721_event);
    }
}

fn handle_erc721_mint(erc721_event: EmittedEvent) {}

fn handle_erc721_burn(erc721_event: EmittedEvent) {}

fn handle_erc721_transfer(erc721_event: EmittedEvent) {}

fn handle_erc1155_event(erc1155_event: EmittedEvent) {}
