//! BLAKE2b-512: package integrity, signature prehash, catalog pin.

use blake2::{Blake2b512, Digest};

/// BLAKE2b with a 64-byte digest.
pub fn blake2b512(buf: &[u8]) -> [u8; 64] {
    let mut out = [0u8; 64];
    out.copy_from_slice(&Blake2b512::digest(buf));
    out
}

/// Lowercase hex of [`blake2b512`] (128 chars).
pub fn blake2b512_hex(buf: &[u8]) -> String {
    blake2b512(buf).iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blake2b512_abc() {
        // RFC 7693 / BLAKE2b-512("abc") known vector, first 8 bytes of the 64-byte digest
        let out = blake2b512(b"abc");
        assert_eq!(&out[..8], &[0xba, 0x80, 0xa5, 0x3f, 0x98, 0x1c, 0x4d, 0x0d]);
    }

    #[test]
    fn blake2b512_hex_abc() {
        let h = blake2b512_hex(b"abc");
        assert_eq!(h.len(), 128);
        assert!(h.starts_with("ba80a53f981c4d0d"));
    }
}
