use md5_hex_helper::md5_hex;
use revised_sdk::baseline::{check_cmd, ntfs_hint};
use revised_sdk::local;
use std::path::{Path, PathBuf};

mod md5_hex_helper {
    use md5::{Digest, Md5};
    pub fn md5_hex(b: &[u8]) -> String {
        Md5::digest(b).iter().map(|x| format!("{x:02x}")).collect()
    }
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rsdk-b-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// rootfs fixture with two files + the matching md5 list.
fn fixture(name: &str) -> (PathBuf, PathBuf, String) {
    let root = tmp(name);
    let (rfs, proj) = (root.join("rootfs"), root.join("proj"));
    std::fs::create_dir_all(rfs.join("etc")).unwrap();
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::write(rfs.join("etc/a"), b"aaa").unwrap();
    std::fs::write(rfs.join("b"), b"bbb").unwrap();
    let list = format!("{}  /etc/a\n{}  /b\n", md5_hex(b"aaa"), md5_hex(b"bbb"));
    (rfs, proj, list)
}

fn paths(r: &revised_sdk::report::Report, k: &str) -> Vec<String> {
    r.data[k]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn all_match_ok() {
    let (rfs, proj, list) = fixture("ok");
    let r = check_cmd(&proj, &rfs, &list);
    assert!(r.ok, "{r:?}");
    assert_eq!(r.data["checked"], 2);
    assert!(paths(&r, "missing").is_empty() && paths(&r, "mismatch").is_empty());
}

#[test]
fn one_mismatch_fails() {
    let (rfs, proj, list) = fixture("mm");
    std::fs::write(rfs.join("b"), b"changed").unwrap();
    let r = check_cmd(&proj, &rfs, &list);
    assert!(!r.ok);
    assert_eq!(r.exit_code(), 1);
    assert_eq!(paths(&r, "mismatch"), ["/b"]);
    assert!(r.errors.iter().any(|f| f.file.as_deref() == Some("/b")));
}

#[test]
fn missing_file_fails() {
    let (rfs, proj, list) = fixture("miss");
    std::fs::remove_file(rfs.join("etc/a")).unwrap();
    let r = check_cmd(&proj, &rfs, &list);
    assert!(!r.ok);
    assert_eq!(paths(&r, "missing"), ["/etc/a"]);
}

#[test]
fn records_path_on_ok() {
    let (rfs, proj, list) = fixture("rec");
    local::save(
        &proj,
        &local::Local {
            unit_host: Some("h".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(check_cmd(&proj, &rfs, &list).ok);
    let l = local::load(&proj);
    assert!(l.baseline.is_some());
    assert_eq!(l.unit_host.as_deref(), Some("h"), "other keys survive");
    // failed check does not record
    let (rfs2, proj2, list2) = fixture("rec2");
    std::fs::remove_file(rfs2.join("b")).unwrap();
    assert!(!check_cmd(&proj2, &rfs2, &list2).ok);
    assert!(local::load(&proj2).baseline.is_none());
}

#[test]
fn ntfs_hint_flags_windows_drives() {
    for p in [r"C:\data\rootfs", "c:/data", "/c/data/rootfs", "/mnt/d/x"] {
        let f = ntfs_hint(Path::new(p)).unwrap_or_else(|| panic!("{p}"));
        assert!(f.msg.contains("POSIX file system"), "{}", f.msg);
        assert_eq!(f.level, revised_sdk::report::Level::Warning);
    }
    assert!(ntfs_hint(Path::new("/home/u/rootfs")).is_none());
    assert!(ntfs_hint(Path::new("/mnt/data")).is_none());
}

#[test]
fn local_roundtrip_keeps_unknown_keys() {
    let d = tmp("local");
    std::fs::create_dir_all(d.join(".revised")).unwrap();
    std::fs::write(
        d.join(".revised/local.toml"),
        "# c\nextra = \"keep\"\nunit_host = \"old\"\n",
    )
    .unwrap();
    let mut l = local::load(&d);
    assert_eq!(l.unit_host.as_deref(), Some("old"));
    l.unit_host = Some("new".into());
    l.baseline = Some(PathBuf::from("/a \"q\"/b"));
    local::save(&d, &l).unwrap();
    let s = std::fs::read_to_string(d.join(".revised/local.toml")).unwrap();
    assert!(s.contains("extra = \"keep\"") && s.contains("# c"), "{s}");
    assert_eq!(local::load(&d).baseline, l.baseline);
    assert_eq!(local::load(&d).unit_host.as_deref(), Some("new"));
    assert!(!tmp("nofile").join("x").exists() && local::load(&tmp("nofile")).baseline.is_none());
}
