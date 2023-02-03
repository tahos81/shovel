use starknet::core::types::FieldElement;

/// Trait for converting a `FieldElement` to a string
pub trait ToUtf8String {
    fn to_utf8_string(&self) -> String;
}

/// Converts a `FieldElement` to a string by converting the bytes to ascii
/// and trimming the null bytes.
impl ToUtf8String for FieldElement {
    fn to_utf8_string(&self) -> String {
        let chars = self.to_bytes_be().into_iter().filter(|&v| v != 0_u8).collect();

        log::debug!("{:?}", &chars);

        String::from_utf8(chars).unwrap_or_default()
    }
}

/// Converts a `Vec<FieldElement>` aka a Cairo string to UTF8 string
impl ToUtf8String for Vec<FieldElement> {
    fn to_utf8_string(&self) -> String {
        // First element is the string length
        let chars: Vec<u8> = self
            .iter()
            .skip(1)
            .flat_map(|felt| {
                let felt_bytes = felt.to_bytes_be();
                felt_bytes.into_iter().filter(|&v| v != 0_u8).collect::<Vec<u8>>()
            })
            .collect();
        String::from_utf8(chars).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use starknet::macros::felt;

    use super::*;

    #[test]
    fn felt_to_string() {
        let encoded = felt!("0x7a65747375");
        let decoded = encoded.to_utf8_string();

        assert_eq!(decoded, "zetsu");
    }

    #[test]
    fn felt_array_to_string() {
        let encoded = vec![felt!("0x2"), felt!("0x7a65747375"), felt!("0x626f6969")];
        let decoded = encoded.to_utf8_string();

        assert_eq!(decoded, "zetsuboii");
    }
}
