/// Decode a string of hex literals into a vector of bytes.
pub fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        Err("Hex string must have an even length".to_string())
    } else {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| format!("{e}")))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_hex() {
        assert_eq!(decode_hex("48656c6c6f").unwrap(), b"Hello");
    }

    #[test]
    fn test_decode_hex_empty() {
        assert_eq!(decode_hex("").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn test_decode_hex_odd_length() {
        let result = decode_hex("48656c6c6");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Hex string must have an even length");
    }

    #[test]
    fn test_decode_hex_invalid_char() {
        let result = decode_hex("48656c6g6f");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("invalid digit found in string")
        );
    }

    #[test]
    fn test_decode_hex_uppercase() {
        assert_eq!(decode_hex("48656C6C6F").unwrap(), b"Hello");
    }

    #[test]
    fn test_decode_hex_zero_bytes() {
        assert_eq!(decode_hex("0000").unwrap(), vec![0, 0]);
    }

    #[test]
    fn test_decode_hex_all_ff() {
        assert_eq!(decode_hex("ffff").unwrap(), vec![255, 255]);
    }
}
