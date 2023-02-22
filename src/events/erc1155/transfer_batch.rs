use crate::{common::types::CairoUint256, events::context::Event};
use color_eyre::eyre::Result;
use starknet::core::types::FieldElement;

pub struct Erc1155TransferBatch {
    pub sender: FieldElement,
    pub recipient: FieldElement,
    pub transfers: Vec<(CairoUint256, CairoUint256)>,
}

pub async fn run<Database>(
    event_context: &Event<'_, '_, Database>,
) -> Result<Erc1155TransferBatch> {
    let contract_address = event_context.contract_address();
    let block_id = event_context.block_id();
    let block_number = event_context.block_number();
    let event_data = event_context.data();

    let sender = event_data[1];
    let recipient = event_data[2];

    // Get the length of the token ids array
    let token_length: u32 = event_data[3].try_into().unwrap();
    let token_length = token_length as usize;

    // This is index difference between token id and corresponding amount in the event data array
    let amount_delta = token_length * 2 + 1;

    // Zip token ids and amounts together
    let transfers: Vec<(FieldElement, FieldElement)> = event_data[4..(3 + amount_delta)]
        .chunks(2)
        .map(|chunk| CairoUint256::new(chunk[0], chunk[1]))
        .zip(
            event_data[(4 + amount_delta)..]
                .chunks(2)
                .map(|chunk| CairoUint256::new(chunk[0], chunk[1])),
        )
        .collect();

    Ok(Erc1155TransferBatch { sender, recipient, transfers })
}
