use starknet::{
    core::types::BlockId,
    macros::felt,
    providers::{Provider, SequencerGatewayProvider},
};

pub async fn run() {
    let provider = SequencerGatewayProvider::starknet_alpha_goerli_2();

    let block = provider.get_block(BlockId::Latest).await.unwrap();
    block
        .transaction_receipts
        .iter()
        .flat_map(|tx_receipt| &tx_receipt.events)
        .filter(|event| {
            event.keys.contains(&felt!(
                "0x99cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9"
            ))
        })
        .for_each(|event| {
            dbg!(event);
        });
}
