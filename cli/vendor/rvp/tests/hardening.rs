//! Fresh-review findings (Important 1-5): duplicates, canonical cores, formatVersion,
//! package name safety, spec MUST rules.
mod common;
use common::{fixture, pkg_copy};
use rvp::pack::{check_signable, gunzip, gzip, pack_dir, unpack_mem};
use rvp::ustar::{ustar_read, ustar_write, UstarEntry};
use serde_json::Value;

fn entry(name: &str, data: &[u8]) -> UstarEntry {
    UstarEntry {
        name: name.into(),
        data: data.to_vec(),
        is_dir: false,
    }
}

fn manifest_value() -> Value {
    serde_json::from_slice(&std::fs::read(fixture("pkg/manifest.json")).unwrap()).unwrap()
}

/// One-file core (`payload/x`, gzipped) from a manifest value with `files` reduced to that file.
fn core_from(m: Value) -> Vec<u8> {
    gzip(&tar_from(m)).unwrap()
}

/// [`core_from`] before gzip.
fn tar_from(mut m: Value) -> Vec<u8> {
    let f = &mut m["files"].as_array_mut().unwrap()[0];
    f["payload"] = "payload/x".into();
    let first = f.clone();
    m["files"] = Value::Array(vec![first]);
    ustar_write(&[
        entry("manifest.json", &serde_json::to_vec(&m).unwrap()),
        entry("payload/x", b"x"),
    ])
    .unwrap()
}

fn err_of(core: &[u8]) -> String {
    unpack_mem(core).unwrap_err().to_string()
}

fn write_manifest(dir: &std::path::Path, m: &Value) {
    std::fs::write(dir.join("manifest.json"), serde_json::to_vec(m).unwrap()).unwrap();
}

/// Rewrite an octal header field of member 0 and fix up the checksum.
fn patch_header(core: &mut [u8], range: std::ops::Range<usize>, text: &[u8]) {
    core[range.clone()][..text.len()].copy_from_slice(text);
    let sum: u64 = core[..512]
        .iter()
        .enumerate()
        .map(|(i, &b)| {
            if (148..156).contains(&i) {
                32
            } else {
                b as u64
            }
        })
        .sum();
    let s = format!("{sum:06o}\0 ");
    core[148..156].copy_from_slice(s.as_bytes());
}

#[test]
fn test_baseline_one_file_core_is_ok() {
    // guards the helpers: every rejection below is about the one thing it changes
    unpack_mem(&core_from(manifest_value())).unwrap();
}

#[test]
fn test_reject_duplicate_member() {
    let m = tar_from(manifest_value());
    let body = &m[..m.len() - 1024]; // manifest.json + payload/x="x", no end blocks
    let dup = ustar_write(&[entry("payload/x", b"BBBB")]).unwrap();
    let core = [body, &dup[..dup.len() - 1024], &[0u8; 1024]].concat();
    assert!(ustar_read(&core)
        .unwrap_err()
        .to_string()
        .contains("duplicate"));
    assert!(err_of(&gzip(&core).unwrap()).contains("duplicate"));
}

#[test]
fn test_reject_non_canonical_core() {
    let good = gunzip(&pack_dir(&fixture("pkg")).unwrap()).unwrap();
    let mut core = good.clone();
    patch_header(&mut core, 100..108, b"0000777");
    assert!(ustar_read(&core).is_ok(), "probe must stay a valid tar");
    assert!(err_of(&gzip(&core).unwrap()).contains("canonical"));

    let mut core = good.clone();
    patch_header(&mut core, 136..148, b"00000000001");
    assert!(err_of(&gzip(&core).unwrap()).contains("canonical"));
}

#[test]
fn test_signable_requires_canonical_manifest() {
    let good = pack_dir(&fixture("pkg")).unwrap();
    check_signable(&good).unwrap();
    // same package, manifest member re-laid-out (leading space): readable, not signable
    let (m, entries) = unpack_mem(&good).unwrap();
    let mut members = vec![entry(
        "manifest.json",
        &[b" ".as_slice(), &m.to_canonical_json().unwrap()].concat(),
    )];
    members.extend(entries);
    let core = gzip(&ustar_write(&members).unwrap()).unwrap();
    assert!(unpack_mem(&core).is_ok());
    let e = check_signable(&core).unwrap_err().to_string();
    assert!(e.contains("canonical"), "{e}");
}

#[test]
fn test_reject_format_version_not_1() {
    for v in [0, 2] {
        let mut m = manifest_value();
        m["formatVersion"] = v.into();
        assert!(err_of(&core_from(m.clone())).contains("formatVersion"));
        let d = pkg_copy(&format!("fv{v}"));
        write_manifest(&d, &m);
        let e = pack_dir(&d).unwrap_err().to_string();
        assert!(e.contains("formatVersion"), "{e}");
    }
}

#[test]
fn test_reject_unsafe_package_name() {
    for bad in [
        "../../../etc",
        "",
        ".",
        "..",
        "a/b",
        "Big",
        "-x",
        &"a".repeat(65),
    ] {
        let mut m = manifest_value();
        m["name"] = bad.into();
        let e = err_of(&core_from(m));
        assert!(e.contains("name"), "{bad:?}: {e}");
    }
    let mut m = manifest_value();
    m["depends"] = "../x".into();
    assert!(err_of(&core_from(m)).contains("name"));
    // allowed shapes
    for ok in ["bootspeed", "aa-touch", "x1.2_y"] {
        let mut m = manifest_value();
        m["name"] = ok.into();
        unpack_mem(&core_from(m)).unwrap();
    }
}

#[test]
fn test_reject_wrong_target_cmu() {
    let mut m = manifest_value();
    m["targetCMU"] = "99.99.999X".into();
    assert!(err_of(&core_from(m)).contains("targetCMU"));
}

#[test]
fn test_reject_new_file_not_whole_file() {
    let mut m = manifest_value();
    m["files"][0]["type"] = "new".into();
    m["files"][0]["apply"] = "3way".into();
    let e = err_of(&core_from(m));
    assert!(e.contains("whole-file"), "{e}");
}
