//! Two-tier trust: an offline root key signs `keys.json`, which endorses the release keys.
//! Signatures are then checked by the endorsed key whose id they carry ([`crate::verify::verify_any`]).

use serde::Deserialize;

use crate::verify::{verify, PubKey};
use crate::RvpError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySet {
    pub seq: u64,
    pub keys: Vec<PubKey>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Doc {
    format: u32,
    seq: u64,
    keys: Vec<String>,
}

impl KeySet {
    /// A set of exactly `pk` (seq 0): tests and `rvp-verify`.
    pub fn single(pk: PubKey) -> KeySet {
        KeySet {
            seq: 0,
            keys: vec![pk],
        }
    }

    /// Verify `json` against the detached root signature `minisig`, then parse it. A `seq`
    /// lower than `last_seq` (the highest ever accepted) is refused; equal is fine.
    pub fn from_signed(
        json: &[u8],
        minisig: &str,
        root: &PubKey,
        last_seq: Option<u64>,
    ) -> Result<KeySet, RvpError> {
        verify(json, minisig, root)?;
        let d: Doc = serde_json::from_slice(json)
            .map_err(|e| RvpError::Verify(format!("keys.json: {e}")))?;
        if d.format != 1 {
            return Err(RvpError::Verify(format!(
                "keys.json: unsupported format {}",
                d.format
            )));
        }
        if last_seq.is_some_and(|l| d.seq < l) {
            return Err(RvpError::Verify(format!(
                "keys.json seq {} is older than the accepted {}",
                d.seq,
                last_seq.unwrap_or(0)
            )));
        }
        let keys = d
            .keys
            .iter()
            .map(|k| PubKey::parse(k))
            .collect::<Result<_, _>>()?;
        Ok(KeySet { seq: d.seq, keys })
    }
}
