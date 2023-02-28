use crate::{
    common::{starknet_constants::ZERO_FELT, types::CairoUint256},
    db::postgres::process::ProcessEvent,
    events::context::Event,
    rpc::metadata::{contract, token},
};
use async_trait::async_trait;
use color_eyre::eyre::Result;
use starknet::{core::types::FieldElement, providers::jsonrpc::models::BlockId};
use token::TokenMetadata;

pub struct Erc721Transfer {
    pub sender: FieldElement,
    pub recipient: FieldElement,
    pub token_id: CairoUint256,
    pub contract_address: FieldElement,
    pub block_number: u64,
}

#[async_trait]
impl ProcessEvent for Erc721Transfer {
    async fn process(
        &mut self,
        ctx: &Event<'_, '_>,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        if self.sender == ZERO_FELT {
            processors::handle_mint(&self, ctx, transaction).await
        } else {
            processors::handle_transfer(&self, ctx, transaction).await
        }
    }
}

pub async fn run(
    ctx: &Event<'_, '_>,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<()> {
    let contract_address = ctx.contract_address();
    let block_number = ctx.block_number();
    let event_data = ctx.data();

    let sender = event_data[0];
    let recipient = event_data[1];
    let token_id = CairoUint256::new(event_data[2], *event_data.get(3).unwrap_or(&ZERO_FELT));

    Erc721Transfer { sender, recipient, token_id, contract_address, block_number }
        .process(ctx, transaction)
        .await
}

mod processors {
    use super::*;

    pub async fn handle_mint(
        event: &Erc721Transfer,
        ctx: &Event<'_, '_>,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        let block_id = BlockId::Number(event.block_number);
        let block_number = i64::try_from(event.block_number).unwrap();

        let token_uri =
            token::get_erc721_uri(event.contract_address, &block_id, ctx.rpc(), event.token_id)
                .await;
        let metadata_result = token::get_token_metadata(&token_uri).await;
        let metadata = match metadata_result {
            Ok(metadata) => metadata,
            Err(_) => TokenMetadata::default(),
        };

        // Check contract metadata
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
            let symbol = contract::get_symbol(event.contract_address, &block_id, ctx.rpc()).await;

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

        // Insert Erc721 data
        let inserted_id = sqlx::query!(
            r#"
                INSERT INTO erc721_data(
                    contract_address,
                    token_id_low,
                    token_id_high,
                    latest_owner,
                    token_uri,
                    last_updated_block)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING id
            "#,
            event.contract_address.to_string(),
            event.token_id.low.to_string(),
            event.token_id.high.to_string(),
            event.recipient.to_string(),
            token_uri,
            i64::try_from(event.block_number).unwrap()
        )
        .fetch_one(&mut *transaction)
        .await?
        .id;

        // Add address to owners
        sqlx::query!(
            r#"
                INSERT INTO erc721_owners(erc721_id, owner, block)
                VALUES($1, $2, $3)
            "#,
            inserted_id,
            event.recipient.to_string(),
            block_number
        )
        .execute(&mut *transaction)
        .await?;

        Ok(())
    }

    pub async fn handle_transfer(
        event: &Erc721Transfer,
        ctx: &Event<'_, '_>,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<()> {
        let block_number = i64::try_from(event.block_number).unwrap();

        // Find the ERC721 entry with given contract address and id
        let erc721_id = sqlx::query!(
            r#"
            SELECT id
            FROM erc721_data
            WHERE
                contract_address = $1 AND
                token_id_low = $2 AND
                token_id_high = $3
            "#,
            event.contract_address.to_string(),
            event.token_id.low.to_string(),
            event.token_id.high.to_string(),
        )
        .fetch_one(&mut *transaction)
        .await?
        .id;

        // Update latest owner
        sqlx::query!(
            r#"
                UPDATE erc721_data
                SET latest_owner = $1, last_updated_block = $2
                WHERE id = $3
            "#,
            event.recipient.to_string(),
            block_number,
            erc721_id,
        )
        .execute(&mut *transaction)
        .await?;

        // Update owners list
        sqlx::query!(
            r#"
                INSERT INTO erc721_owners(erc721_id, owner, block)
                VALUES($1, $2, $3)
            "#,
            erc721_id,
            event.recipient.to_string(),
            block_number
        )
        .execute(&mut *transaction)
        .await?;

        Ok(())
    }
}
