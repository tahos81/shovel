pub mod erc1155;
pub mod erc721;

use async_trait::async_trait;
use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
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

#[async_trait]
pub trait IntoEventDiff {
    async fn into_event_diff(self) -> EventDiff;
}

pub struct Erc721Diff {
    contract_address: FieldElement,
    token_id: FieldElement,
    new_owner: String,
    timestamp: u64,
}

pub struct Erc1155Diff {
    contract_address: FieldElement,
    token_id: FieldElement,
    // ( positive, amount )
    balance_change: (bool, FieldElement),
}

pub enum EventDiff {
    Erc721(Erc721Diff),
    Erc1155(Erc1155Diff),
}

pub struct BlockDiff {
    batch_id: u64,
    events: Vec<EventDiff>,
}

impl BlockDiff {
    pub fn new(batch_id: u64, events: Vec<EventDiff>) -> Self {
        Self { batch_id, events }
    }

    pub fn batch_id(&self) -> u64 {
        self.batch_id
    }

    pub fn events(&self) -> &[EventDiff] {
        self.events.as_ref()
    }
}

pub struct EventHandler<'a> {
    rpc: &'a JsonRpcClient<HttpTransport>,
    pool: &'a Pool<Postgres>,
}

impl<'a> EventHandler<'a> {
    pub fn new(rpc: &JsonRpcClient<HttpTransport>, pool: &Pool<Postgres>) -> Self {
        EventHandler { rpc, pool }
    }

    pub async fn diff_from_events(
        &mut self,
        batch_id: u64,
        events: &[EmittedEvent],
    ) -> Result<BlockDiff> {
        let mut event_diffs = Vec::<EventDiff>::new();

        for event in events {
            match self.diff_from_event(event).await {
                Ok(diff) => event_diffs.push(diff),
                Err(e) => eprintln!("{:?}", e),
            }
        }

        Ok(BlockDiff::new(batch_id, event_diffs))
    }

    pub async fn diff_from_event(&mut self, event: &EmittedEvent) -> Result<EventDiff> {
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
        .fetch_one(&self.pool)
        .await?
        .exists
        .unwrap_or_default();

        if blacklisted {
            color_eyre::eyre::bail!("Contract blacklisted")
        }

        let keys = &event.keys;
        match keys {
            [&TRANSFER_EVENT_KEY, ..] => {
                // Both ERC20 and ERC721 contracts use same event key to represent transfers so
                // we have to check if the contract is ERC721 and blacklist if not so.
                let is_erc721 = contract::is_erc721(contract_address, &block_id, self.rpc).await?;

                if is_erc721 {
                    let erc721_transfer = Erc721Transfer::from(event);
                    erc721_transfer.process(self).await?;
                } else {
                    // Blacklist the non-ERC721 token
                    sqlx::query!(
                        r#"
                        INSERT INTO blacklisted_contracts(address)
                        VALUES ($1)
                        ON CONFLICT DO NOTHING
                    "#,
                        contract_address.to_string()
                    )
                    .execute(&self.pool)
                    .await?;
                }
            }
            [&TRANSFER_SINGLE_EVENT_KEY, ..] => {
                todo!()
            }
            [&TRANSFER_BATCH_EVENT_KEY, ..] => {
                todo!()
            }
            _ => {}
        }
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
