use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_compact::{KeyPair, Seed};
use rvp::hash::blake2b512;
use rvp::trust::KeySet;
use rvp::verify::{verify_any, verify_digest_any, PubKey};

struct Key {
    kp: KeyPair,
    id: [u8; 8],
}

fn key(n: u8) -> Key {
    Key {
        kp: KeyPair::from_seed(Seed::new([n; 32])),
        id: [n; 8],
    }
}

impl Key {
    fn pubkey_line(&self) -> String {
        STANDARD.encode([b"Ed".as_slice(), &self.id, &*self.kp.pk].concat())
    }
    fn pubkey(&self) -> PubKey {
        PubKey::parse(&self.pubkey_line()).unwrap()
    }
    /// minisign prehashed (ED) signature text.
    fn sign(&self, msg: &[u8]) -> String {
        let sig = self.kp.sk.sign(blake2b512(msg), None);
        let tc = "t";
        let global = self.kp.sk.sign([&*sig, tc.as_bytes()].concat(), None);
        format!(
            "untrusted comment: x\n{}\ntrusted comment: {tc}\n{}\n",
            STANDARD.encode([b"ED".as_slice(), &self.id, &*sig].concat()),
            STANDARD.encode(*global)
        )
    }
}

fn keys_json(format: u32, seq: u64, keys: &[String]) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"format": format, "seq": seq, "keys": keys})).unwrap()
}

fn signed(root: &Key, seq: u64, rel: &Key) -> (Vec<u8>, String) {
    let j = keys_json(1, seq, &[rel.pubkey_line()]);
    let s = root.sign(&j);
    (j, s)
}

#[test]
fn chain_ok() {
    let (root, rel) = (key(1), key(2));
    let (j, s) = signed(&root, 5, &rel);
    let ks = KeySet::from_signed(&j, &s, &root.pubkey(), None).unwrap();
    assert_eq!(ks.seq, 5);
    assert_eq!(ks.keys, vec![rel.pubkey()]);
    let core = b"catalog";
    verify_any(core, &rel.sign(core), &ks).unwrap();
    verify_digest_any(&blake2b512(core), &rel.sign(core), &ks).unwrap();
}

#[test]
fn wrong_root_refused() {
    let (root, other, rel) = (key(1), key(3), key(2));
    let (j, s) = signed(&other, 5, &rel);
    assert!(KeySet::from_signed(&j, &s, &root.pubkey(), None).is_err());
}

#[test]
fn seq_rollback_refused() {
    let (root, rel) = (key(1), key(2));
    let (j, s) = signed(&root, 5, &rel);
    let e = KeySet::from_signed(&j, &s, &root.pubkey(), Some(6))
        .unwrap_err()
        .to_string();
    assert!(e.contains("older"), "{e}");
    KeySet::from_signed(&j, &s, &root.pubkey(), Some(5)).unwrap();
}

#[test]
fn unlisted_key_refused() {
    let (root, rel, rogue) = (key(1), key(2), key(4));
    let (j, s) = signed(&root, 1, &rel);
    let ks = KeySet::from_signed(&j, &s, &root.pubkey(), None).unwrap();
    let e = verify_any(b"c", &rogue.sign(b"c"), &ks)
        .unwrap_err()
        .to_string();
    assert!(e.contains("not endorsed"), "{e}");
}

#[test]
fn bad_format_refused() {
    let (root, rel) = (key(1), key(2));
    let j = keys_json(2, 1, &[rel.pubkey_line()]);
    assert!(KeySet::from_signed(&j, &root.sign(&j), &root.pubkey(), None).is_err());
    let j = keys_json(1, 1, &["AAAA".into()]);
    assert!(KeySet::from_signed(&j, &root.sign(&j), &root.pubkey(), None).is_err());
}
