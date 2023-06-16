use crate::{
    common::types::CairoUint256,
    events::{EventDiff, EventHandler, HexFieldElement, IntoEventDiff},
    rpc::metadata::contract,
};
use async_trait::async_trait;
use color_eyre::eyre;
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{
        models::{BlockId, EmittedEvent},
        HttpTransport, JsonRpcClient,
    },
};

pub struct Erc1155TransferSingle {
    pub sender: HexFieldElement,
    pub recipient: HexFieldElement,
    pub token_id: CairoUint256,
    pub amount: CairoUint256,
    pub contract_address: HexFieldElement,
    pub block_number: u64,
}

impl Erc1155TransferSingle {
    pub fn new(
        sender: FieldElement,
        recipient: FieldElement,
        token_id: CairoUint256,
        amount: CairoUint256,
        contract_address: FieldElement,
        block_number: u64,
    ) -> Self {
        Erc1155TransferSingle {
            sender: HexFieldElement(sender),
            recipient: HexFieldElement(recipient),
            token_id,
            amount,
            contract_address: HexFieldElement(contract_address),
            block_number,
        }
    }
}

#[async_trait]
impl IntoEventDiff for Erc1155TransferSingle {
    async fn into_event_diff(self, handler: &EventHandler<'_>) -> eyre::Result<EventDiff> {
        processors::get_diff(&self, handler.rpc, handler.pool)
    }
}

impl From<&EmittedEvent> for Erc1155TransferSingle {
    fn from(event: &EmittedEvent) -> Self {
        let contract_address = event.from_address;
        let block_number = event.block_number;
        let event_data = &event.data;

        let sender = event_data[1];
        let recipient = event_data[2];
        let token_id = CairoUint256::new(event_data[3], event_data[4]);
        let amount = CairoUint256::new(event_data[5], event_data[6]);

        Erc1155TransferSingle::new(
            sender,
            recipient,
            token_id,
            amount,
            contract_address,
            block_number,
        )
    }
}

mod processors {
    use sqlx::{Pool, Postgres};

    use crate::events::{Erc1155SingleDiff, EventDiff};
    use crate::rpc::metadata::token::TokenMetadata;

    use super::super::super::super::rpc::metadata::token;
    use super::{
        contract, eyre, BlockId, Erc1155TransferSingle, FieldElement, HttpTransport, JsonRpcClient,
    };

    pub async fn get_diff(
        event: &Erc1155TransferSingle,
        rpc: &JsonRpcClient<HttpTransport>,
        pool: &Pool<Postgres>,
    ) -> eyre::Result<EventDiff> {
        let block_id = BlockId::Number(event.block_number);
        let block_number = i64::try_from(event.block_number).unwrap();

        // If we're minting the token, make a call for metadata
        if event.sender == FieldElement::ZERO {
            // Check if contract metadata exists
            let contract_metadata_exists = sqlx::query!(
                r#"
                    SELECT EXISTS (
                        SELECT * 
                        FROM contract_metadata 
                        WHERE
                            contract_address = $1 AND
                            contract_type = 'ERC1155'
                    )
                "#,
                event.contract_address.to_string()
            )
            .fetch_one(&pool)
            .await?
            .exists
            .unwrap_or_default();

            if !contract_metadata_exists {
                // Query name and symbol, then insert contract metadata
                let name = contract::get_name(event.contract_address.0, &block_id, rpc).await;
                let symbol = contract::get_symbol(event.contract_address.0, &block_id, rpc).await;

                sqlx::query!(
                    r#"
                    INSERT INTO contract_metadata(
                        contract_address,
                        contract_type,
                        name,
                        symbol,
                        last_updated_block)
                    VALUES ($1, 'ERC1155', $2, $3, $4)
                "#,
                    event.contract_address.to_string(),
                    name,
                    symbol,
                    block_number
                )
                .execute(&pool)
                .await?;
            }

            // Unlike ERC721 tokens, ERC1155 tokens can be minted more than once,
            // so we have to check for existing token metadata before fetching it
            let token_metadata_exists = sqlx::query!(
                r#"
                    SELECT EXISTS (
                        SELECT * 
                        FROM token_metadata
                        WHERE 
                            contract_address = $1 AND
                            contract_type = 'ERC1155' AND
                            token_id_low = $2 AND
                            token_id_high = $3
                    )
                "#,
                event.contract_address.to_string(),
                event.token_id.low.to_string(),
                event.token_id.high.to_string(),
            )
            .fetch_one(&pool)
            .await?
            .exists
            .unwrap_or_default();

            if !token_metadata_exists {
                fetch_and_insert_metadata(event, rpc, &pool).await?;
            }
        }

        Ok(Erc1155SingleDiff {
            contract_address: event.contract_address.to_string(),
            sender: event.sender.to_string(),
            recipient: event.recipient.to_string(),
            token_id: (event.token_id.low.to_string(), event.token_id.high.to_string()),
            amount: (event.amount.low.to_string(), event.amount.high.to_string()),
            block_number: event.block_number,
        })
    }

    async fn fetch_and_insert_metadata(
        event: &Erc1155TransferSingle,
        rpc: &JsonRpcClient<HttpTransport>,
        pool: &Pool<Postgres>,
    ) -> eyre::Result<()> {
        let block_id = BlockId::Number(event.block_number);

        let token_uri =
            token::get_erc1155_uri(event.contract_address.0, &block_id, rpc, event.token_id).await;
        let metadata_result = token::get_token_metadata(&token_uri).await;
        let metadata = match metadata_result {
            Ok(metadata) => metadata,
            Err(_) => TokenMetadata::default(),
        };

        // Insert token_metadata
        let token_metadata_id = sqlx::query!(
            r#"
                INSERT INTO token_metadata(
                    contract_address,
                    contract_type,
                    token_id_low,
                    token_id_high,
                    -- Metadata
                    image,
                    image_data,
                    external_url,
                    description,
                    name,
                    background_color,
                    animation_url,
                    youtube_url)
                VALUES($1, 'ERC1155', $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                RETURNING id
            "#,
            event.contract_address.to_string(),
            event.token_id.low.to_string(),
            event.token_id.high.to_string(),
            metadata.image,
            metadata.image_data,
            metadata.external_url,
            metadata.description,
            metadata.name,
            metadata.background_color,
            metadata.animation_url,
            metadata.youtube_url
        )
        .fetch_one(&pool)
        .await?
        .id;

        // Insert token metadata attributes
        if let Some(attributes) = metadata.attributes {
            for attribute in &attributes {
                sqlx::query!(
                    r#"
                    INSERT INTO token_metadata_attributes(
                        token_metadata_id,
                        value,
                        display_type,
                        trait_type)
                    VALUES($1, $2, $3, $4)
                "#,
                    token_metadata_id,
                    serde_json::to_string(&attribute.value)
                        .expect("attribute.value serialize failed"),
                    serde_json::to_string(&attribute.display_type)
                        .expect("attribute.display_type serialize failed"),
                    serde_json::to_string(&attribute.trait_type)
                        .expect("attribute.trait_type serialize failed")
                )
                .execute(&pool)
                .await?;
            }
        }

        Ok(())
    }
}
