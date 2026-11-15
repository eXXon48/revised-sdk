mod common;
use common::fixture;
use rvp::pack::pack_dir;
use rvp::verify::{verify, PubKey};

fn read(p: &str) -> Vec<u8> {
    std::fs::read(fixture(p)).unwrap()
}

fn text(p: &str) -> String {
    String::from_utf8(read(p)).unwrap()
}

fn pk() -> PubKey {
    PubKey::parse(&text("test.pub")).unwrap()
}

/// Writes tests/fixtures/pkg.core; run by gen-sig-fixtures.sh only.
#[test]
#[ignore]
fn regen_pkg_core() {
    if std::env::var("RVP_REGEN").is_ok() {
        std::fs::write(fixture("pkg.core"), pack_dir(&fixture("pkg")).unwrap()).unwrap();
    }
}

#[test]
fn test_fixture_core_matches_pkg() {
    // the signed fixture core is the current deterministic core of fixtures/pkg
    assert_eq!(read("pkg.core"), pack_dir(&fixture("pkg")).unwrap());
}

#[test]
fn test_verify_ok() {
    verify(&read("pkg.core"), &text("pkg.core.minisig"), &pk()).unwrap();
}

#[test]
fn test_reject_tampered_core() {
    let mut core = read("pkg.core");
    let mid = core.len() / 2;
    core[mid] ^= 1;
    assert!(verify(&core, &text("pkg.core.minisig"), &pk()).is_err());
}

#[test]
fn test_reject_wrong_keyid() {
    let e = verify(&read("pkg.core"), &text("pkg.core.otherkey.minisig"), &pk())
        .unwrap_err()
        .to_string();
    assert!(e.contains("key id"), "{e}");
}

#[test]
fn test_reject_tampered_tc() {
    let sig = text("pkg.core.minisig").replace("revised-approved", "revised-approveD");
    let e = verify(&read("pkg.core"), &sig, &pk())
        .unwrap_err()
        .to_string();
    assert!(e.contains("trusted comment"), "{e}");
}

#[test]
fn test_reject_legacy_algo() {
    let e = verify(&read("pkg.core"), &text("pkg.core.legacy.minisig"), &pk())
        .unwrap_err()
        .to_string();
    assert!(e.contains("prehashed"), "{e}");
}

#[test]
fn test_reject_garbage_never_panics() {
    let core = read("pkg.core");
    for s in [
        "",
        "x",
        "untrusted comment: a\n",
        "a\nb\nc\nd\n",
        "a\n!!!!\ntrusted comment: x\n===\n",
    ] {
        assert!(verify(&core, s, &pk()).is_err());
    }
    assert!(PubKey::parse("").is_err());
    assert!(PubKey::parse("untrusted comment: x\nAAAA\n").is_err());
}

#[test]
fn test_verify_container() {
    use rvp::container::container_write;
    use rvp::verify::verify_container;
    let core = read("pkg.core");
    let sig = read("pkg.core.minisig");
    verify_container(&container_write(&core, Some(&sig)).unwrap(), &pk()).unwrap();
    let e = verify_container(&container_write(&core, None).unwrap(), &pk())
        .unwrap_err()
        .to_string();
    assert!(e.contains("unsigned"), "{e}");
}

#[test]
fn read_unsigned_roundtrip() {
    use rvp::container::container_write;
    use rvp::hash::blake2b512;
    use rvp::verify::read_unsigned_stream;
    let core = read("pkg.core");
    let mut out = vec![];
    let d = read_unsigned_stream(&container_write(&core, None).unwrap()[..], &mut out).unwrap();
    assert_eq!(out, core);
    assert_eq!(d, blake2b512(&core));
}

#[test]
fn read_unsigned_rejects_signed() {
    use rvp::container::container_write;
    use rvp::verify::read_unsigned_stream;
    let signed = container_write(&read("pkg.core"), Some(&read("pkg.core.minisig"))).unwrap();
    let e = read_unsigned_stream(&signed[..], &mut std::io::sink()).unwrap_err().to_string();
    assert!(e.contains("package is signed"), "{e}");
}
