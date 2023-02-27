use crate::{
    common::types::CairoUint256, db::postgres::process::ProcessEvent, events::context::Event,
};
use async_trait::async_trait;
use color_eyre::eyre::Result;
use starknet::core::types::FieldElement;

use super::transfer_single::Erc1155TransferSingle;

pub struct Erc1155TransferBatch {
    pub sender: FieldElement,
    pub recipient: FieldElement,
    pub transfers: Vec<(CairoUint256, CairoUint256)>,
    pub contract_address: FieldElement,
    pub block_number: u64,
}

#[async_trait]
impl ProcessEvent for Erc1155TransferBatch {
    async fn process(&mut self, ctx: &Event<'_, '_>) -> Result<()> {
        for transfer in self.transfers {
            Erc1155TransferSingle {
                sender: self.sender,
                recipient: self.recipient,
                token_id: transfer.0,
                amount: transfer.1,
                contract_address: self.contract_address,
                block_number: self.block_number,
            }
            .process(ctx)
            .await?;
        }

        Ok(())
    }
}

pub async fn run(ctx: &Event<'_, '_>) -> Result<()> {
    let contract_address = ctx.contract_address();
    let block_id = ctx.block_id();
    let block_number = ctx.block_number();
    let event_data = ctx.data();

    let sender = event_data[1];
    let recipient = event_data[2];

    // Get the length of the token ids array
    let token_length: u32 = event_data[3].try_into().unwrap();
    let token_length = token_length as usize;

    // This is index difference between token id and corresponding amount in the event data array
    let amount_delta = token_length * 2 + 1;

    // Zip token ids and amounts together
    let transfers: Vec<(CairoUint256, CairoUint256)> = event_data[4..(3 + amount_delta)]
        .chunks(2)
        .map(|chunk| CairoUint256::new(chunk[0], chunk[1]))
        .zip(
            event_data[(4 + amount_delta)..]
                .chunks(2)
                .map(|chunk| CairoUint256::new(chunk[0], chunk[1])),
        )
        .collect();

    Erc1155TransferBatch { sender, recipient, transfers, contract_address, block_number }
        .process(ctx)
        .await;

    Ok(())
}
