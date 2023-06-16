pub mod erc1155;
pub mod erc721;

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

#[derive(Debug)]
pub enum Event {
    Erc721Transfer(Erc721Transfer),
    Erc1155TransferSingle(Erc1155TransferSingle),
    Erc1155TransferBatch(Erc1155TransferBatch),
}

#[derive(Debug)]
pub struct EventBatch {
    batch_id: u64,
    events: Vec<Event>,
}

#[allow(dead_code)]
impl EventBatch {
    pub fn new(batch_id: u64, events: Vec<Event>) -> Self {
        Self { batch_id, events }
    }

    pub fn batch_id(&self) -> u64 {
        self.batch_id
    }

    pub fn events(&self) -> &[Event] {
        self.events.as_ref()
    }

    pub fn into_events(self) -> Vec<Event> {
        self.events
    }
}

pub struct EventHandler<'a> {
    rpc: &'a JsonRpcClient<HttpTransport>,
    pool: &'a Pool<Postgres>,
}

impl<'a> EventHandler<'a> {
    pub fn new(rpc: &'a JsonRpcClient<HttpTransport>, pool: &'a Pool<Postgres>) -> Self {
        EventHandler { rpc, pool }
    }

    pub async fn read_events(
        &self,
        batch_id: u64,
        events: &[EmittedEvent],
    ) -> eyre::Result<EventBatch> {
        let mut event_diffs = Vec::<Event>::new();

        for event in events {
            match self.read_event(event).await {
                Ok(diff) => event_diffs.push(diff),
                Err(e) => eprintln!("{:?}", e),
            }
        }

        Ok(EventBatch::new(batch_id, event_diffs))
    }

    pub async fn read_event(&self, event: &EmittedEvent) -> eyre::Result<Event> {
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
        .fetch_one(self.pool)
        .await?
        .exists
        .unwrap_or_default();

        if blacklisted {
            eyre::bail!("Contract blacklisted")
        }

        let keys = &event.keys;
        if keys.contains(&TRANSFER_EVENT_KEY) {
            // Both ERC20 and ERC721 contracts use same event key to represent transfers so
            // we have to check if the contract is ERC721 and blacklist if not so.
            let is_erc721 = contract::is_erc721(contract_address, &block_id, self.rpc).await?;

            if is_erc721 {
                Ok(Event::Erc721Transfer(Erc721Transfer::from(event)))
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
                .execute(self.pool)
                .await?;

                eyre::bail!("No matching event")
            }
        } else if keys.contains(&TRANSFER_SINGLE_EVENT_KEY) {
            Ok(Event::Erc1155TransferSingle(Erc1155TransferSingle::from(event)))
        } else if keys.contains(&TRANSFER_BATCH_EVENT_KEY) {
            Ok(Event::Erc1155TransferBatch(Erc1155TransferBatch::from(event)))
        } else {
            eyre::bail!("No matching event")
        }
    }
}

#[derive(Debug, Clone)]
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
