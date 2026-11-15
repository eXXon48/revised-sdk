mod common;
use common::{fixture, pkg_copy};
use rvp::hash::blake2b512_hex;
use rvp::manifest::Manifest;
use rvp::pack::{pack_dir, unpack_mem};
use rvp::ustar::{ustar_write, UstarEntry};

#[test]
fn test_pack_reproducible() {
    let a = pack_dir(&fixture("pkg")).unwrap();
    let b = pack_dir(&fixture("pkg")).unwrap();
    assert_eq!(a, b);
    assert_eq!(blake2b512_hex(&a), blake2b512_hex(&b));
}

#[test]
fn test_pack_unpack() {
    let core = pack_dir(&fixture("pkg")).unwrap();
    let (m, entries) = unpack_mem(&core).unwrap();
    assert_eq!(m.name, "bootspeed");
    assert_eq!(m.files.len(), 2);
    for f in &m.files {
        let on_disk = std::fs::read(fixture("pkg").join(f.payload.as_ref().unwrap())).unwrap();
        let e = entries
            .iter()
            .find(|e| Some(&e.name) == f.payload.as_ref())
            .unwrap();
        assert_eq!(e.data, on_disk);
    }
}

#[test]
fn test_reject_missing_payload() {
    let d = pkg_copy("missing");
    std::fs::remove_file(d.join("payload/www/app.js")).unwrap();
    let e = pack_dir(&d).unwrap_err().to_string();
    assert!(e.contains("payload/www/app.js"), "{e}");
}

#[test]
fn test_reject_orphan_payload() {
    let d = pkg_copy("orphan");
    std::fs::write(d.join("payload/stray.txt"), b"x").unwrap();
    let e = pack_dir(&d).unwrap_err().to_string();
    assert!(e.contains("payload/stray.txt"), "{e}");
}

#[test]
fn test_reject_empty_files() {
    let d = pkg_copy("empty");
    let mut m = Manifest::parse(&std::fs::read(d.join("manifest.json")).unwrap()).unwrap();
    m.files.clear();
    std::fs::write(d.join("manifest.json"), m.to_canonical_json().unwrap()).unwrap();
    std::fs::remove_dir_all(d.join("payload")).unwrap();
    let e = pack_dir(&d).unwrap_err().to_string();
    assert!(e.contains("no files"), "{e}");
}

fn core_with_payload_path(payload: &str, live: &str) -> Vec<u8> {
    let mut m = Manifest::parse(&std::fs::read(fixture("pkg/manifest.json")).unwrap()).unwrap();
    m.files.truncate(1);
    m.files[0].payload = Some(payload.into());
    m.files[0].live = live.into();
    rvp::pack::gzip(&ustar_write(&[
        UstarEntry {
            name: "manifest.json".into(),
            data: m.to_canonical_json().unwrap(),
            is_dir: false,
        },
        UstarEntry {
            name: payload.into(),
            data: b"x".to_vec(),
            is_dir: false,
        },
    ])
    .unwrap())
    .unwrap()
}

#[test]
fn test_reject_path_escape() {
    let core = core_with_payload_path("payload/../../etc/x", "/jci/x");
    let e = unpack_mem(&core).unwrap_err().to_string();
    assert!(e.contains(".."), "{e}");
    let core = core_with_payload_path("/etc/x", "/jci/x");
    assert!(unpack_mem(&core).is_err());
    // live path escape / relative (jailed) live path on a modify
    let core = core_with_payload_path("payload/x", "/jci/../../etc/shadow");
    assert!(unpack_mem(&core).is_err());
    let core = core_with_payload_path("payload/x", "jci/x");
    assert!(unpack_mem(&core).is_err());
    // the clean variant is accepted (proves the rejections are about the path)
    let core = core_with_payload_path("payload/x", "/jci/x");
    assert!(unpack_mem(&core).is_ok());
}
