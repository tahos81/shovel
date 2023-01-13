#![allow(unused)]

use std::ops::{Add, Neg, Sub};

use serde::{Deserialize, Serialize};
use starknet::core::types::FieldElement;

// felt!(2**128)
pub const CAIRO_UINT128_SHIFT: FieldElement = FieldElement::from_mont([
    18446744073700081665,
    17407,
    18446744073709551584,
    576460752142434320,
]);

// felt!(2**128 - 1)
pub const CAIRO_UINT128_ALL_ONES: FieldElement = FieldElement::from_mont([
    18446744073700081697,
    17407,
    18446744073709551584,
    576460752142434864,
]);

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct CairoUint256 {
    pub low: FieldElement,
    pub high: FieldElement,
}

impl PartialEq for CairoUint256 {
    fn eq(&self, other: &Self) -> bool {
        self.low == other.low && self.high == other.high
    }
}

impl Add for CairoUint256 {
    type Output = Self;

    /// Returns the sum of the two `CairoUint256`s.
    fn add(self, other: Self) -> Self {
        let sum_low = self.low + other.low;

        let carry_low =
            if sum_low.ge(&CAIRO_UINT128_SHIFT) { FieldElement::ONE } else { FieldElement::ZERO };

        let sum_high = self.high + other.high + carry_low;

        let carry_high =
            if sum_high.ge(&CAIRO_UINT128_SHIFT) { FieldElement::ONE } else { FieldElement::ZERO };

        Self::new(
            sum_low - carry_low * CAIRO_UINT128_SHIFT,
            sum_high - carry_high * CAIRO_UINT128_SHIFT,
        )
    }
}

impl Neg for CairoUint256 {
    type Output = Self;

    /// Returns the two's complement of the `CairoUint256`.
    fn neg(self) -> Self {
        let u256_not = self.not();
        u256_not + Self::new(FieldElement::ONE, FieldElement::ZERO)
    }
}

impl Sub for CairoUint256 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        self + (-other)
    }
}

impl CairoUint256 {
    pub const ZERO: Self = CairoUint256 { low: FieldElement::ZERO, high: FieldElement::ZERO };

    pub const ONE: Self = CairoUint256 { low: FieldElement::ONE, high: FieldElement::ZERO };

    /// Creates a new CairoUint256 from the given low and high `FieldElement`s.
    pub fn new(low: FieldElement, high: FieldElement) -> Self {
        CairoUint256 { low, high }
    }

    /// Returns the bitwise NOT of the `CairoUint256`.
    /// This is equivalent to `felt!(2**256 - 1) - self`.
    pub fn not(self) -> Self {
        Self::new(CAIRO_UINT128_ALL_ONES - self.low, CAIRO_UINT128_ALL_ONES - self.high)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        let a = CairoUint256::new(FieldElement::from(1u32), FieldElement::from(0u32));
        let b = CairoUint256::new(FieldElement::from(1u32), FieldElement::from(0u32));
        let c = a + b;
        assert_eq!(c.low, FieldElement::from(2u32));
        assert_eq!(c.high, FieldElement::from(0u32));
    }

    #[test]
    fn test_add_overflow() {
        let a = CairoUint256::new(CAIRO_UINT128_ALL_ONES, CAIRO_UINT128_ALL_ONES);
        let b = CairoUint256::new(FieldElement::ONE, FieldElement::ZERO);
        assert_eq!(a + b, CairoUint256::new(FieldElement::ZERO, FieldElement::ZERO));
    }

    #[test]
    fn test_sub() {
        let a = CairoUint256::new(FieldElement::from(100u32), FieldElement::from(20u32));
        let b = CairoUint256::new(FieldElement::from(40u32), FieldElement::from(5u32));
        assert_eq!(a - b, CairoUint256::new(FieldElement::from(60u32), FieldElement::from(15u32)));
    }
}
