use mongodb::Database;
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{
        models::{BlockId, EmittedEvent},
        HttpTransport, JsonRpcClient,
    },
};

pub struct Event<'a, 'b> {
    pub event: &'b EmittedEvent,
    pub rpc: &'a JsonRpcClient<HttpTransport>,
    pub db: &'a Database,
}

impl<'a, 'b> Event<'a, 'b> {
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
}
