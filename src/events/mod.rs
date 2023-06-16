pub mod erc1155;
pub mod erc721;

use async_trait::async_trait;
use color_eyre::eyre;
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
    rpc::metadata::contract,
};

use self::{
    erc1155::{transfer_batch::Erc1155TransferBatch, transfer_single::Erc1155TransferSingle},
    erc721::transfer::Erc721Transfer,
};

#[async_trait]
pub trait IntoEventDiff {
    async fn into_event_diff(self, handler: &EventHandler<'_>) -> EventDiff;
}

pub struct Erc721Diff {
    contract_address: FieldElement,
    token_id: FieldElement,
    new_owner: String,
    block_number: u64,
}

pub struct Erc1155SingleDiff {
    contract_address: String,
    sender: String,
    recipient: String,
    token_id: (String, String),
    amount: (String, String),
    block_number: u64,
}

pub struct Erc1155BatchDiff {
    contract_address: String,
    sender: String,
    recipient: String,
    token_ids: Vec<(String, String)>,
    amounts: Vec<(String, String)>,
    block_number: u64,
}

pub enum EventDiff {
    Erc721(Erc721Diff),
    Erc1155Single(Erc1155SingleDiff),
    Erc1155Batch(Erc1155BatchDiff),
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
    ) -> eyre::Result<BlockDiff> {
        let mut event_diffs = Vec::<EventDiff>::new();

        for event in events {
            match self.diff_from_event(event).await {
                Ok(diff) => event_diffs.push(diff),
                Err(e) => eprintln!("{:?}", e),
            }
        }

        Ok(BlockDiff::new(batch_id, event_diffs))
    }

    pub async fn diff_from_event(&mut self, event: &EmittedEvent) -> eyre::Result<EventDiff> {
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
            eyre::bail!("Contract blacklisted")
        }

        let keys = &event.keys;
        match keys {
            [&TRANSFER_EVENT_KEY, ..] => {
                // Both ERC20 and ERC721 contracts use same event key to represent transfers so
                // we have to check if the contract is ERC721 and blacklist if not so.
                let is_erc721 = contract::is_erc721(contract_address, &block_id, self.rpc).await?;

                if is_erc721 {
                    Erc721Transfer::from(event).into_event_diff(&self).await?
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

                    eyre::bail!("No matching event")
                }
            }
            [&TRANSFER_SINGLE_EVENT_KEY, ..] => {
                Erc1155TransferSingle::from(event).into_event_diff(&self).await?
            }
            [&TRANSFER_BATCH_EVENT_KEY, ..] => {
                Erc1155TransferBatch::from(event).into_event_diff(&self).await?
            }
            _ => {
                eyre::bail!("No matching event")
            }
        }
    }
}

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
