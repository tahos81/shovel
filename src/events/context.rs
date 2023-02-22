use sqlx::Pool;
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{
        models::{BlockId, EmittedEvent},
        HttpTransport, JsonRpcClient,
    },
};

pub struct Event<'a, 'b, Database> {
    event: &'b EmittedEvent,
    rpc: &'a JsonRpcClient<HttpTransport>,
    db: &'a Pool<Database>,
}

impl<'a, 'b, Database> Event<'a, 'b, Database> {
    pub fn new(
        event: &'b EmittedEvent,
        rpc: &'a JsonRpcClient<HttpTransport>,
        db: &'a Database,
    ) -> Self {
        Self { event, rpc, db }
    }

    pub fn contract_address(&self) -> FieldElement {
        self.event.from_address
    }

    pub fn block_id(&self) -> BlockId {
        BlockId::Number(self.event.block_number)
    }

    pub fn block_number(&self) -> u64 {
        self.event.block_number
    }

    pub fn data(&self) -> &Vec<FieldElement> {
        &self.event.data
    }

    pub fn rpc(&self) -> &JsonRpcClient<HttpTransport> {
        self.rpc
    }

    pub fn db(&self) -> &Pool<Database> {
        self.db
    }
}