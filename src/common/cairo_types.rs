use serde::{Deserialize, Serialize};
use starknet::core::types::FieldElement;

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct CairoUint256 {
    pub low: FieldElement,
    pub high: FieldElement,
}

impl CairoUint256 {
    pub fn new(low: FieldElement, high: FieldElement) -> Self {
        CairoUint256 { low, high }
    }
}
