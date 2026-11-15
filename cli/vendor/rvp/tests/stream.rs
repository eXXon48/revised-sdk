//! gzip core (Package Format §3, 2026-11-13) + streaming verify / unpack.
mod common;
use common::fixture;
use rvp::container::container_write;
use rvp::hash::blake2b512;
use rvp::pack::{gunzip, gzip, pack_dir, unpack_mem, unpack_stream};
use rvp::ustar::ustar_read;
use rvp::verify::{verify_stream, PubKey};

fn read(p: &str) -> Vec<u8> {
    std::fs::read(fixture(p)).unwrap()
}

fn pk() -> PubKey {
    PubKey::parse(&String::from_utf8(read("test.pub")).unwrap()).unwrap()
}

#[test]
fn test_core_is_deterministic_gzip() {
    let a = pack_dir(&fixture("pkg")).unwrap();
    assert_eq!(a, pack_dir(&fixture("pkg")).unwrap());
    assert_eq!(&a[..3], &[0x1f, 0x8b, 8], "gzip magic + deflate");
    assert_eq!(a[3], 0, "no FNAME/FCOMMENT/FEXTRA/FHCRC");
    assert_eq!(&a[4..8], &[0, 0, 0, 0], "mtime 0");
    assert_eq!(a[9], 255, "OS unknown");
    // inside: the canonical ustar, and gzip(gunzip(core)) is the core again
    let tar = gunzip(&a).unwrap();
    ustar_read(&tar).unwrap();
    assert_eq!(gzip(&tar).unwrap(), a);
}

#[test]
fn test_unpack_stream_matches_unpack_mem() {
    let core = pack_dir(&fixture("pkg")).unwrap();
    let (m, entries) = unpack_mem(&core).unwrap();
    let mut got = vec![];
    let m2 = unpack_stream(&core[..], |mm, name, r| {
        assert_eq!(mm.name, m.name);
        let mut d = vec![];
        r.read_to_end(&mut d).unwrap();
        got.push((name.to_string(), d));
        Ok(())
    })
    .unwrap();
    assert_eq!(m2, m);
    let want: Vec<_> = entries.into_iter().map(|e| (e.name, e.data)).collect();
    assert_eq!(got, want);
}

#[test]
fn test_unpack_stream_callback_may_skip_data() {
    // a reader that ignores member data (layers rebuild skipping jail files) still validates
    let core = pack_dir(&fixture("pkg")).unwrap();
    unpack_stream(&core[..], |_, _, _| Ok(())).unwrap();
}

#[test]
fn test_reject_raw_tar_core() {
    let tar = gunzip(&pack_dir(&fixture("pkg")).unwrap()).unwrap();
    assert!(unpack_mem(&tar).is_err());
}

#[test]
fn test_reject_trailing_bytes_after_gzip() {
    let mut core = pack_dir(&fixture("pkg")).unwrap();
    core.push(0);
    let e = unpack_mem(&core).unwrap_err().to_string();
    assert!(e.contains("trailing"), "{e}");
    assert!(gunzip(&core).is_err());
}

#[test]
fn test_reject_truncated_gzip() {
    let core = pack_dir(&fixture("pkg")).unwrap();
    assert!(unpack_mem(&core[..core.len() - 4]).is_err());
}

#[test]
fn test_verify_stream_copies_core_and_returns_digest() {
    let core = read("pkg.core");
    let sig = read("pkg.core.minisig");
    let rvp = container_write(&core, Some(&sig)).unwrap();
    let mut out = vec![];
    let d = verify_stream(&rvp[..], &pk(), &mut out).unwrap();
    assert_eq!(d, blake2b512(&core));
    assert_eq!(out, core);
}

#[test]
fn test_verify_stream_rejects_tamper_and_unsigned() {
    let mut core = read("pkg.core");
    let sig = read("pkg.core.minisig");
    let unsigned = container_write(&core, None).unwrap();
    let e = verify_stream(&unsigned[..], &pk(), &mut std::io::sink()).unwrap_err().to_string();
    assert!(e.contains("unsigned"), "{e}");
    let n = core.len() - 1;
    core[n] ^= 1;
    let bad = container_write(&core, Some(&sig)).unwrap();
    assert!(verify_stream(&bad[..], &pk(), &mut std::io::sink()).is_err());
}
