pub mod erc1155;
pub mod erc721;

use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{
        models::{BlockId, EmittedEvent},
        HttpTransport, JsonRpcClient,
    },
};
use std::str::FromStr;

use crate::{
    common::starknet_constants::{
        TRANSFER_BATCH_EVENT_KEY, TRANSFER_EVENT_KEY, TRANSFER_SINGLE_EVENT_KEY,
    },
    db::postgres::process::ProcessEvent,
    rpc::metadata::contract,
};

use self::{
    erc1155::{transfer_batch::Erc1155TransferBatch, transfer_single::Erc1155TransferSingle},
    erc721::transfer::Erc721Transfer,
};

pub struct EventHandler<'a, 'b> {
    rpc: &'a JsonRpcClient<HttpTransport>,
    transaction: &'b mut sqlx::Transaction<'a, sqlx::Postgres>,
}

impl<'a, 'b> EventHandler<'a, 'b> {
    pub fn new(
        rpc: &'a JsonRpcClient<HttpTransport>,
        transaction: &'b mut sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> Self {
        EventHandler { rpc, transaction }
    }

    pub async fn handle(&mut self, event: &EmittedEvent) -> Result<()> {
        let block_id = BlockId::Number(event.block_number);
        let contract_address = event.from_address;

        let blacklisted = sqlx::query!(
            r#"
                SELECT EXISTS (
                    SELECT 1
                    FROM blacklisted_contracts
                    WHERE address = $1
                )
            "#,
            contract_address.to_string()
        )
        .fetch_one(&mut *self.transaction)
        .await?
        .exists
        .unwrap_or_default();

        if blacklisted {
            return Ok(());
        }

        let keys = &event.keys;
        if keys.contains(&TRANSFER_EVENT_KEY) {
            // Both ERC20 and ERC721 use the same event key
            let is_erc721 = contract::is_erc721(contract_address, &block_id, self.rpc).await?;

            if is_erc721 {
                let erc721_transfer = Erc721Transfer::from(event);
                erc721_transfer.process(self).await?;
            } else {
                sqlx::query!(
                    r#"
                        INSERT INTO blacklisted_contracts(address)
                        VALUES ($1)
                    "#,
                    contract_address.to_string()
                )
                .execute(&mut *self.transaction)
                .await?;
            }
        } else if keys.contains(&TRANSFER_SINGLE_EVENT_KEY) {
            let erc1155_transfer_single = Erc1155TransferSingle::from(event);
            erc1155_transfer_single.process(self).await?;
        } else if keys.contains(&TRANSFER_BATCH_EVENT_KEY) {
            let erc1155_transfer_batch = Erc1155TransferBatch::from(event);
            erc1155_transfer_batch.process(self).await?;
        }

        Ok(())
    }
}

// TODO: Move to a more suiting folder

#[derive(Debug)]
pub struct HexFieldElement(FieldElement);

impl Serialize for HexFieldElement {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{:#x}", self.0))
    }
}

impl<'de> Deserialize<'de> for HexFieldElement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let inner = FieldElement::from_str(&value).map_err(serde::de::Error::custom)?;
        Ok(Self(inner))
    }
}

impl ToString for HexFieldElement {
    fn to_string(&self) -> String {
        format!("{:#x}", self.0)
    }
}

impl From<HexFieldElement> for FieldElement {
    fn from(val: HexFieldElement) -> Self {
        val.0
    }
}

impl PartialEq<FieldElement> for HexFieldElement {
    fn eq(&self, other: &FieldElement) -> bool {
        self.0 == *other
    }
}
