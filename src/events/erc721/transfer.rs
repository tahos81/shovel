use crate::{
    common::{types::CairoUint256},
    events::{EventDiff, EventHandler, HexFieldElement, IntoEventDiff},
    rpc::metadata::{contract, token},
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
use token::TokenMetadata;

pub struct Erc721Transfer {
    pub sender: HexFieldElement,
    pub recipient: HexFieldElement,
    pub token_id: CairoUint256,
    pub contract_address: HexFieldElement,
    pub block_number: u64,
}

impl Erc721Transfer {
    pub fn new(
        sender: FieldElement,
        recipient: FieldElement,
        token_id: CairoUint256,
        contract_address: FieldElement,
        block_number: u64,
    ) -> Self {
        Erc721Transfer {
            sender: HexFieldElement(sender),
            recipient: HexFieldElement(recipient),
            token_id,
            contract_address: HexFieldElement(contract_address),
            block_number,
        }
    }
}

#[async_trait]
impl IntoEventDiff for Erc721Transfer {
    async fn into_event_diff(self, handler: &EventHandler<'_>) -> eyre::Result<EventDiff> {
        processors::get_diff(&self, handler.rpc, handler.pool)
    }
}

impl From<&EmittedEvent> for Erc721Transfer {
    fn from(event: &EmittedEvent) -> Self {
        let contract_address = event.from_address;
        let block_number = event.block_number;
        let event_data = &event.data;

        let sender = event.data[0];
        let recipient = event_data[1];
        let token_id =
            CairoUint256::new(event_data[2], *event_data.get(3).unwrap_or(&FieldElement::ZERO));

        Erc721Transfer::new(sender, recipient, token_id, contract_address, block_number)
    }
}

mod processors {
    use sqlx::{Pool, Postgres};

    use crate::events::Erc721Diff;

    use super::{
        contract, eyre, token, BlockId, Erc721Transfer, EventDiff, HttpTransport, JsonRpcClient,
        TokenMetadata,
    };

    pub async fn get_diff(
        event: &Erc721Transfer,
        rpc: &JsonRpcClient<HttpTransport>,
        pool: &Pool<Postgres>,
    ) -> eyre::Result<EventDiff> {
        let block_id = BlockId::Number(event.block_number);
        let token_uri = fetch_and_insert_metadata(event, rpc, &pool).await?;

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
        .fetch_one(&pool)
        .await?
        .exists
        .unwrap_or_default();

        if !contract_metadata_exists {
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
                    VALUES ($1, 'ERC721', $2, $3, $4)
                "#,
                event.contract_address.to_string(),
                name,
                symbol,
                event.block_number
            )
            .execute(&pool)
            .await?;
        }

        Ok(Erc721Diff {
            contract_address: event.contract_address.to_string(),
            token_id: (event.token_id.low.to_string(), event.token_id.high.to_string()),
            new_owner: event.recipient.to_string(),
            block_number: event.block_number,
        })
    }

    async fn fetch_and_insert_metadata(
        event: &Erc721Transfer,
        rpc: &JsonRpcClient<HttpTransport>,
        pool: &Pool<Postgres>,
    ) -> eyre::Result<String> {
        let block_id = BlockId::Number(event.block_number);

        let token_uri =
            token::get_erc721_uri(event.contract_address.0, &block_id, rpc, event.token_id).await;
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
                VALUES($1, 'ERC721', $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
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

        Ok(token_uri)
    }
}
