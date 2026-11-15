//! minisign signature verify over a core (Package Format §4, Signing & Pipeline §2).
//!
//! Prehashed only: `sig_algo == "ED"`, `sig` = Ed25519 over BLAKE2b-512(core);
//! `global_sig` = Ed25519 over `sig || trusted_comment`. Legacy `"Ed"` is rejected.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_compact::{PublicKey, Signature};

use crate::container::{container_head, FLAG_SIGNED};
use crate::hash::blake2b512;
use crate::RvpError;

const UNTRUSTED: &str = "untrusted comment:";
const TRUSTED: &str = "trusted comment: ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PubKey {
    pub key_id: [u8; 8],
    pub pk: [u8; 32],
}

fn err(m: impl Into<String>) -> RvpError {
    RvpError::Verify(m.into())
}

fn b64(s: &str, what: &str) -> Result<Vec<u8>, RvpError> {
    STANDARD
        .decode(s.trim())
        .map_err(|_| err(format!("{what} is not valid base64")))
}

fn lines(text: &str) -> Vec<&str> {
    text.lines().map(|l| l.trim_end_matches('\r')).collect()
}

impl PubKey {
    /// Accepts a minisign `.pub` file (untrusted comment + key line) or the bare key line.
    pub fn parse(text: &str) -> Result<PubKey, RvpError> {
        let line = lines(text)
            .into_iter()
            .find(|l| !l.is_empty() && !l.starts_with(UNTRUSTED))
            .ok_or_else(|| err("public key: no key line"))?;
        let raw = b64(line, "public key")?;
        if raw.len() != 42 || &raw[..2] != b"Ed" {
            return Err(err("public key: not a minisign Ed25519 key"));
        }
        let mut key_id = [0u8; 8];
        let mut pk = [0u8; 32];
        key_id.copy_from_slice(&raw[2..10]);
        pk.copy_from_slice(&raw[10..42]);
        Ok(PubKey { key_id, pk })
    }
}

fn ed_verify(pk: &[u8; 32], msg: &[u8], sig: &[u8]) -> bool {
    match (PublicKey::from_slice(pk), Signature::from_slice(sig)) {
        (Ok(pk), Ok(sig)) => pk.verify(msg, &sig).is_ok(),
        _ => false,
    }
}

/// Ok iff `minisig_text` is a valid prehashed minisign signature of `core` by `pk`.
pub fn verify(core: &[u8], minisig_text: &str, pk: &PubKey) -> Result<(), RvpError> {
    verify_digest(&blake2b512(core), minisig_text, pk)
}

/// [`verify`] given `BLAKE2b-512(core)` instead of the core (prehashed: the digest is all the
/// signature covers, so a streamed core needs no buffering).
pub fn verify_digest(digest: &[u8; 64], minisig_text: &str, pk: &PubKey) -> Result<(), RvpError> {
    let ls = lines(minisig_text);
    let ls: &[&str] = match ls.as_slice() {
        [body @ .., ""] => body,
        all => all,
    };
    let [untrusted, sig_line, tc_line, global_line] = ls else {
        return Err(err("signature: expected 4 lines"));
    };
    if !untrusted.starts_with(UNTRUSTED) {
        return Err(err("signature: missing untrusted comment line"));
    }
    let raw = b64(sig_line, "signature")?;
    if raw.len() != 74 {
        return Err(err("signature: wrong length"));
    }
    match &raw[..2] {
        b"ED" => {}
        b"Ed" => return Err(err("signature is legacy (Ed); prehashed (ED) is required")),
        _ => {
            return Err(err(
                "signature: unknown algorithm; prehashed (ED) is required",
            ))
        }
    }
    if raw[2..10] != pk.key_id {
        return Err(err("signature key id does not match the public key id"));
    }
    let sig = &raw[10..74];
    if !ed_verify(&pk.pk, digest, sig) {
        return Err(err("signature does not match the core"));
    }
    let tc = tc_line
        .strip_prefix(TRUSTED)
        .ok_or_else(|| err("signature: missing trusted comment line"))?;
    let global = b64(global_line, "global signature")?;
    if global.len() != 64 {
        return Err(err("global signature: wrong length"));
    }
    if !ed_verify(&pk.pk, &[sig, tc.as_bytes()].concat(), &global) {
        return Err(err("trusted comment signature is invalid"));
    }
    Ok(())
}

/// Streams an `.rvp`: require SIGNED, hash the core while copying it to `core_out`, verify the
/// signature over that hash. Returns `BLAKE2b-512(core)` (the catalog pin). Memory is one chunk;
/// a write error from `core_out` (e.g. a size cap) aborts.
pub fn verify_stream(
    mut rvp: impl std::io::Read,
    pk: &PubKey,
    core_out: &mut dyn std::io::Write,
) -> Result<[u8; 64], RvpError> {
    use blake2::{Blake2b512, Digest};
    let h = container_head(&mut rvp)?;
    let sig = match h.sig {
        Some(s) if h.flags & FLAG_SIGNED != 0 => s,
        _ => return Err(err("package is unsigned")),
    };
    let text = std::str::from_utf8(&sig).map_err(|_| err("signature: not UTF-8"))?;
    let (mut hasher, mut buf) = (Blake2b512::new(), vec![0u8; 64 << 10]);
    loop {
        let n = rvp.read(&mut buf).map_err(|e| err(format!("core: {e}")))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        core_out
            .write_all(&buf[..n])
            .map_err(|e| err(format!("core: {e}")))?;
    }
    let mut d = [0u8; 64];
    d.copy_from_slice(&hasher.finalize());
    verify_digest(&d, text, pk)?;
    Ok(d)
}

/// [`verify_stream`] for an UNSIGNED `.rvp` (developer mode): hashes the core while copying it to
/// `core_out`, no signature check. A signed container is refused (it goes through
/// [`verify_stream`]). Returns `BLAKE2b-512(core)`.
pub fn read_unsigned_stream(
    mut rvp: impl std::io::Read,
    core_out: &mut dyn std::io::Write,
) -> Result<[u8; 64], RvpError> {
    use blake2::{Blake2b512, Digest};
    if container_head(&mut rvp)?.flags & FLAG_SIGNED != 0 {
        return Err(err("package is signed"));
    }
    let (mut hasher, mut buf) = (Blake2b512::new(), vec![0u8; 64 << 10]);
    loop {
        let n = rvp.read(&mut buf).map_err(|e| err(format!("core: {e}")))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        core_out
            .write_all(&buf[..n])
            .map_err(|e| err(format!("core: {e}")))?;
    }
    let mut d = [0u8; 64];
    d.copy_from_slice(&hasher.finalize());
    Ok(d)
}

/// Demux an `.rvp` held in memory, require SIGNED, verify the embedded signature over its core.
pub fn verify_container(rvp: &[u8], pk: &PubKey) -> Result<(), RvpError> {
    verify_stream(rvp, pk, &mut std::io::sink()).map(|_| ())
}
