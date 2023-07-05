use crate::{common::types::CairoUint256, events::HexFieldElement};
use starknet::core::types::{EmittedEvent, FieldElement};

#[derive(Debug, Clone)]
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

pub mod process_event {
    use async_trait::async_trait;
    use color_eyre::eyre;
    use starknet::{
        core::types::{BlockId, FieldElement},
        providers::jsonrpc::{HttpTransport, JsonRpcClient},
    };

    use crate::{
        common::types::CairoUint256,
        db::postgres::process::ProcessEvent,
        rpc::metadata::{
            contract,
            token::{self, TokenMetadata},
        },
    };

    use super::Erc1155TransferSingle;

    #[async_trait]
    impl ProcessEvent for Erc1155TransferSingle {
        async fn process(
            &self,
            rpc: &'static JsonRpcClient<HttpTransport>,
            transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        ) -> eyre::Result<()> {
            if self.sender == FieldElement::ZERO {
                println!("[erc1155] processing mint");
                self::process_mint(self, rpc, transaction).await
            } else {
                println!("[erc1155] processing transfer");
                self::process_transfer(self, transaction).await
            }
        }
    }

    pub async fn process_mint(
        event: &Erc1155TransferSingle,
        rpc: &JsonRpcClient<HttpTransport>,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> eyre::Result<()> {
        let block_id = BlockId::Number(event.block_number);
        let block_number = i64::try_from(event.block_number).unwrap();
        let token_uri = self::fetch_and_insert_metadata(event, rpc, &mut *transaction).await?;
        println!("[process_mint] got uri {:?} for token #{}", token_uri, event.token_id.low);

        // Check contract metadata
        let contract_metadata_id = sqlx::query!(
            r#"
                SELECT id
                FROM contract_metadata 
                WHERE
                    contract_address = $1 AND
                    contract_type = 'ERC1155'
            "#,
            event.contract_address.to_string()
        )
        .fetch_one(&mut *transaction)
        .await
        .map(|record| record.id);

        println!("[process_mint] contract_metadata_exists: {contract_metadata_id:?}");

        // If no contract metadata exists, insert a new record, or use the existing one
        let contract_metadata_id = match contract_metadata_id {
            Ok(id) => id,
            Err(_) => {
                println!("[process_mint] no metadata found, inserting a new one");
                let name = contract::get_name(event.contract_address.0, &block_id, rpc).await;
                let symbol = contract::get_symbol(event.contract_address.0, &block_id, rpc).await;
                println!("[process_mint] name: {}, symbol: {}", &name, &symbol);

                sqlx::query!(
                    r#"
                    INSERT INTO contract_metadata(
                        contract_address,
                        contract_type,
                        name,
                        symbol,
                        last_updated_block)
                    VALUES ($1, 'ERC1155', $2, $3, $4)
                    RETURNING id
                "#,
                    event.contract_address.to_string(),
                    name,
                    symbol,
                    block_number
                )
                .fetch_one(&mut *transaction)
                .await?
                .id
            }
        };

        // Check if ERC1155 token record exists for given token id
        let erc1155_token_exists = sqlx::query!(
            r#"
                SELECT EXISTS(
                    SELECT * FROM erc1155_token
                    WHERE contract_address = $1 AND
                    token_id_low = $2 AND
                    token_id_high = $3
                )
            "#,
            event.contract_address.to_string(),
            event.token_id.low.to_string(),
            event.token_id.high.to_string(),
        )
        .fetch_one(&mut *transaction)
        .await?
        .exists
        .unwrap_or_default();

        // If there's no ERC1155 record, create a new one
        if !erc1155_token_exists {
            sqlx::query!(
                r#"
                    INSERT INTO erc1155_token(
                        contract_id,
                        contract_address,
                        token_id_low,
                        token_id_high,
                        token_uri,
                        last_updated_block
                    )
                    VALUES($1, $2, $3, $4, $5, $6)
                "#,
                contract_metadata_id,
                event.contract_address.to_string(),
                event.token_id.low.to_string(),
                event.token_id.high.to_string(),
                token_uri,
                i64::try_from(event.block_number).unwrap()
            )
            .execute(&mut *transaction)
            .await?;
        }

        Ok(())
    }

    pub async fn process_transfer(
        event: &Erc1155TransferSingle,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> eyre::Result<()> {
        let block_number = i64::try_from(event.block_number).unwrap();

        // Get the corresponding ERC1155 token id
        let token_id = sqlx::query!(
            r#"
                SELECT id
                FROM erc1155_token
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
        .await
        .map(|record| record.id);

        let token_id = match token_id {
            Ok(record) => record,
            Err(_) => eyre::bail!("no matching token in db"),
        };

        // First, update from balance
        let balance_record = sqlx::query!(
            r#"
                SELECT id, balance_low, balance_high
                FROM erc1155_balances
                WHERE erc1155_id = $1 AND
                account = $2
            "#,
            token_id,
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

        // Update to balance
        let balance_record = sqlx::query!(
            r#"
                SELECT id, balance_low, balance_high
                FROM erc1155_balances
                WHERE erc1155_id = $1 AND
                account = $2
            "#,
            token_id,
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
                            erc1155_id,
                            account,
                            balance_low,
                            balance_high,
                            last_updated_block)
                        VALUES ($1, $2, $3, $4, $5)
                    "#,
                    token_id,
                    event.recipient.to_string(),
                    event.amount.low.to_string(),
                    event.amount.high.to_string(),
                    block_number
                )
                .execute(&mut *transaction)
                .await?;
            }
        }

        Ok(())
    }

    async fn fetch_and_insert_metadata(
        event: &Erc1155TransferSingle,
        rpc: &JsonRpcClient<HttpTransport>,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> eyre::Result<String> {
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
        .fetch_one(&mut *transaction)
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
                .execute(&mut *transaction)
                .await?;
            }
        }

        Ok(token_uri)
    }
}
