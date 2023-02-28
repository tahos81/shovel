use crate::{
    common::types::CairoUint256, db::postgres::process::ProcessEvent, events::context::Event,
    rpc::metadata::contract,
};
use async_trait::async_trait;
use color_eyre::eyre::Result;
use starknet::{core::types::FieldElement, providers::jsonrpc::models::BlockId};

pub struct Erc1155TransferSingle {
    pub sender: FieldElement,
    pub recipient: FieldElement,
    pub token_id: CairoUint256,
    pub amount: CairoUint256,
    pub contract_address: FieldElement,
    pub block_number: u64,
}

#[async_trait]
impl ProcessEvent for Erc1155TransferSingle {
    async fn process(
        &mut self,
        ctx: &Event<'_, '_>,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        processors::handle_transfer(&mut self, ctx, transaction).await
    }
}

pub async fn run(
    ctx: &Event<'_, '_>,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<()> {
    let contract_address = ctx.contract_address();
    let block_number = ctx.block_number();
    let event_data = ctx.data();

    let sender = event_data[1];
    let recipient = event_data[2];
    let token_id = CairoUint256::new(event_data[3], event_data[4]);
    let amount = CairoUint256::new(event_data[5], event_data[6]);

    Erc1155TransferSingle { sender, recipient, token_id, amount, contract_address, block_number }
        .process(ctx, transaction)
        .await
}

mod processors {
    use super::*;

    pub async fn handle_transfer(
        event: &Erc1155TransferSingle,
        ctx: &Event<'_, '_>,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        let block_id = BlockId::Number(event.block_number);
        let block_number = i64::try_from(event.block_number).unwrap();

        // First, update from balance
        if event.sender == FieldElement::ZERO {
            // Check if contract metadata exists
            let contract_metadata_exists = sqlx::query!(
                r#"
                    SELECT EXISTS (
                        SELECT * 
                        FROM contract_metadata 
                        WHERE
                            contract_address = $1 AND
                            contract_type = 'ERC721'
                    )
                "#,
                event.contract_address.to_string()
            )
            .fetch_one(&mut *transaction)
            .await?
            .exists
            .unwrap_or_default();

            if !contract_metadata_exists {
                let name = contract::get_name(event.contract_address, &block_id, ctx.rpc()).await;
                let symbol =
                    contract::get_symbol(event.contract_address, &block_id, ctx.rpc()).await;

                sqlx::query!(
                    r#"
                    INSERT INTO contract_metadata(
                        contract_address,
                        contract_type,
                        name,
                        symbol,
                        last_updated_block)
                    VALUES ($1, 'ERC721', $2, $3, $4)
                "#,
                    event.contract_address.to_string(),
                    name,
                    symbol,
                    block_number
                )
                .execute(&mut *transaction)
                .await?;
            }
        } else {
            let balance_record = sqlx::query!(
                r#"
                    SELECT id, balance_low, balance_high
                    FROM erc1155_balances
                    WHERE 
                        contract_address = $1 AND 
                        token_id_low = $2 AND
                        token_id_high = $3 AND
                        account = $4
                "#,
                event.contract_address.to_string(),
                event.token_id.low.to_string(),
                event.token_id.high.to_string(),
                event.sender.to_string()
            )
            .fetch_one(&mut *transaction)
            .await
            .ok();

            match balance_record {
                Some(record) => {
                    let before_balance = CairoUint256::new(
                        FieldElement::from_dec_str(&record.balance_low)
                            .expect("balance_low isn't a felt"),
                        FieldElement::from_dec_str(&record.balance_high)
                            .expect("balance_high isn't a felt"),
                    );
                    let new_balance = before_balance - event.amount;

                    sqlx::query!(
                        r#"
                            UPDATE erc1155_balances
                            SET balance_low = $1, balance_high = $2
                            WHERE id = $3
                        "#,
                        new_balance.low.to_string(),
                        new_balance.high.to_string(),
                        record.id
                    )
                    .execute(&mut *transaction)
                    .await?;
                }
                None => {
                    println!("Impossible state, from balance 0");
                }
            }
        }

        // Update to balance
        let balance_record = sqlx::query!(
            r#"
                SELECT id, balance_low, balance_high
                FROM erc1155_balances
                WHERE 
                    contract_address = $1 AND 
                    token_id_low = $2 AND
                    token_id_high = $3 AND
                    account = $4
            "#,
            event.contract_address.to_string(),
            event.token_id.low.to_string(),
            event.token_id.high.to_string(),
            event.recipient.to_string()
        )
        .fetch_one(&mut *transaction)
        .await
        .ok();

        match balance_record {
            // Update the existing balance
            Some(record) => {
                let before_balance = CairoUint256::new(
                    FieldElement::from_dec_str(&record.balance_low)
                        .expect("balance_low isn't a felt"),
                    FieldElement::from_dec_str(&record.balance_high)
                        .expect("balance_high isn't a felt"),
                );
                let new_balance = before_balance + event.amount;

                // Update the existing balance
                sqlx::query!(
                    r#"
                        UPDATE erc1155_balances
                        SET balance_low = $1, balance_high = $2
                        WHERE id = $3
                    "#,
                    new_balance.low.to_string(),
                    new_balance.high.to_string(),
                    record.id
                )
                .execute(&mut *transaction)
                .await?;
            }
            None => {
                // Insert new balance
                sqlx::query!(
                    r#"
                        INSERT INTO erc1155_balances(
                            contract_address,
                            token_id_low,
                            token_id_high,
                            account,
                            balance_low,
                            balance_high,
                            last_updated_block)
                        VALUES ($1, $2, $3, $4, $5, $6, $7)
                    "#,
                    event.contract_address.to_string(),
                    event.token_id.low.to_string(),
                    event.token_id.high.to_string(),
                    event.recipient.to_string(),
                    event.amount.low.to_string(),
                    event.amount.high.to_string(),
                    block_number
                );
            }
        }

        Ok(())
    }
}
