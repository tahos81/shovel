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
