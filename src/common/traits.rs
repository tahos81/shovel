use starknet::core::types::FieldElement;

/// Trait for converting a FieldElement to a string
pub trait AsciiExt {
    fn to_ascii(&self) -> String;
}

/// Converts a FieldElement to a string by converting the bytes to ascii
/// and trimming the null bytes.
impl AsciiExt for FieldElement {
    fn to_ascii(&self) -> String {
        std::str::from_utf8(&self.to_bytes_be())
            .unwrap_or_default()
            .trim_start_matches('\0')
            .to_string()
    }
}
