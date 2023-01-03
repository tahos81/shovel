use starknet::{
    core::types::{BlockId, Event, FieldElement},
    macros::{felt, felt_hex},
    providers::{self, Provider, SequencerGatewayProvider},
};

/// felt!("0x99cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9");
const ERC721_TRANSFER_EVENT: FieldElement = FieldElement::from_mont([
    10370298062762752593,
    7288672513944573579,
    6148261015514870755,
    242125613396778233,
]);

/// TODO: Update
/// felt!("0xdeadbeef");
const ERC1155_TRANSFER_EVENT: FieldElement = FieldElement::from_mont([
    18446743954159837729,
    18446744073709551615,
    18446744073709551615,
    576458719958287408,
]);

/// TODO: Update
/// felt!("0xdeadbeef");
const ERC1155_TRANSFER_BATCH_EVENT: FieldElement = FieldElement::from_mont([
    18446743954159837729,
    18446744073709551615,
    18446744073709551615,
    576458719958287408,
]);

#[derive(Debug)]
enum TransactionType {
    ERC721,
    ERC1155,
    ERC1155Batch,
    Other,
}

fn get_tx_type(event: &Event) -> TransactionType {
    if event.keys.contains(&ERC721_TRANSFER_EVENT) {
        TransactionType::ERC721
    } else if event.keys.contains(&ERC1155_TRANSFER_EVENT) {
        TransactionType::ERC1155
    } else if event.keys.contains(&ERC1155_TRANSFER_BATCH_EVENT) {
        TransactionType::ERC1155Batch
    } else {
        TransactionType::Other
    }
}

pub async fn run() {
    let provider = SequencerGatewayProvider::starknet_alpha_goerli_2();

    let block = provider.get_block(BlockId::Latest).await.unwrap();
    for receipt in block.transaction_receipts.iter() {
        for event in receipt.events.iter() {
            match get_tx_type(&event) {
                TransactionType::ERC721 => {
                    dbg!(TransactionType::ERC721, event);
                },
                TransactionType::ERC1155 => {
                    dbg!(TransactionType::ERC1155, event);
                },
                TransactionType::ERC1155Batch => {
                    dbg!(TransactionType::ERC1155Batch, event);
                },
                _ => {}
            }
        }
    }
}
