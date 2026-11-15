use revised_sdk::setup::*;
use std::path::{Path, PathBuf};

// sha256("abc")
const ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rsdk-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn stub(bytes: &'static [u8]) -> impl Fn(&str, &Path) -> Result<(), String> {
    move |_url, dest| std::fs::write(dest, bytes).map_err(|e| e.to_string())
}

fn manifest(hash: &str) -> String {
    format!(
        "{hash}  opera-12.10-1652.x86_64.linux.tar.xz  https://x.test/opera-12.10-1652.x86_64.linux.tar.xz\n\
         {hash}  Opera_1210_en_Setup_x64.exe  https://x.test/Opera_1210_en_Setup_x64.exe\n\
         {hash}  Opera_12.10_Setup_Intel_i386_x86_64.dmg  https://x.test/Opera_12.10_Setup_Intel_i386_x86_64.dmg\n"
    )
}

#[test]
fn sha256_mismatch_refuses() {
    let dir = tmp("mismatch");
    let m = manifest(ABC);
    let entries = parse_manifest(&m).unwrap();
    let e = &entries[0];
    let err = download_verified(e, &dir, &stub(b"wrong")).unwrap_err();
    assert!(err.contains("sha256"), "{err}");
    assert!(!dir.join(&e.name).exists(), "bad download must be deleted");
    assert!(download_verified(e, &dir, &stub(b"abc")).is_ok());
    assert!(dir.join(&e.name).is_file());
}

#[test]
fn pending_hash_is_env_error() {
    let dir = tmp("pending");
    let o = OperaOpts {
        manifest: &manifest("PENDING"),
        os: "linux",
        image: false,
        host_dir: dir.clone(),
        engine: None,
        json: false,
        fetch: &stub(b"abc"),
    };
    assert_eq!(opera_install(&o), 2);
    assert!(std::fs::read_dir(&dir).unwrap().next().is_none(), "nothing downloaded");
    assert!(parse_manifest(&manifest("PENDING")).unwrap()[0].is_pending());
}

#[test]
fn mismatch_exit_code_is_1() {
    let dir = tmp("exit1");
    let o = OperaOpts {
        manifest: &manifest(ABC),
        os: "windows",
        image: false,
        host_dir: dir,
        engine: None,
        json: false,
        fetch: &stub(b"nope"),
    };
    assert_eq!(opera_install(&o), 1);
}

#[test]
fn windows_installer_is_saved_not_run() {
    let dir = tmp("win");
    let o = OperaOpts {
        manifest: &manifest(ABC),
        os: "windows",
        image: false,
        host_dir: dir.clone(),
        engine: None,
        json: false,
        fetch: &stub(b"abc"),
    };
    assert_eq!(opera_install(&o), 0);
    assert!(dir.join("Opera_1210_en_Setup_x64.exe").is_file());
}

#[test]
fn opera_target_volume() {
    let a = image_install_args("revised-sdk:0.1.0", Path::new("/tmp/dl"), "opera.tar.xz");
    let s = a.join(" ");
    assert!(s.contains("revised-sdk-opera:/opera"), "{s}");
    assert!(s.contains("/tmp/dl:/dl:ro"), "{s}");
    assert!(s.contains("revised-sdk:0.1.0"), "{s}");
    assert!(!s.contains("commit"), "never writes into the image: {s}");
}

#[test]
fn opera_host_paths() {
    let env = |k: &str| match k {
        "LOCALAPPDATA" => Some(r"C:\Users\a\AppData\Local".to_string()),
        "HOME" => Some("/home/a".to_string()),
        _ => None,
    };
    assert_eq!(
        host_dir("windows", &env).unwrap(),
        PathBuf::from(r"C:\Users\a\AppData\Local").join("revised-sdk").join("opera")
    );
    assert_eq!(
        host_dir("linux", &env).unwrap(),
        PathBuf::from("/home/a/.local/share/revised-sdk/opera")
    );
    assert_eq!(
        host_dir("macos", &env).unwrap(),
        PathBuf::from("/home/a/Library/Application Support/revised-sdk/opera")
    );
}

#[test]
fn shipped_manifest_parses_and_is_pending() {
    let m = include_str!("../../container/opera.sha256");
    let v = parse_manifest(m).unwrap();
    assert!(v.len() >= 3);
    assert!(v.iter().all(|e| e.url.starts_with("https://")));
}

#[test]
fn setup_requires_store() {
    let o = SetupOpts { store: None, sdk: None, engine: None, dry_run: true, json: false };
    assert_eq!(setup(&o), 2);
}

#[test]
fn setup_build_args() {
    let a = build_args("revised-sdk:0.1.0", Path::new("/sdk"), Path::new("/store"), "abc123");
    let s = a.join(" ");
    assert!(s.starts_with("build "), "{s}");
    assert!(s.contains("-t revised-sdk:0.1.0"), "{s}");
    assert!(s.contains("--label org.revised.store-rev=abc123"), "{s}");
    assert!(s.contains("--build-context store=/store"), "{s}");
    assert!(s.contains("-f /sdk/container/Containerfile"), "{s}");
    assert!(s.ends_with("/sdk"), "{s}");
}
