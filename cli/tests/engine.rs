use revised_sdk::engine::*;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

#[test]
fn host_path_windows() {
    assert_eq!(host_path(Path::new(r"C:\Users\a b\p")), "C:/Users/a b/p");
}

#[test]
fn detect_none_is_env_error() {
    let e = detect_in(None, OsStr::new("")).unwrap_err();
    assert!(e.contains("install podman or docker"), "{e}");
}

#[test]
fn image_missing_message() {
    // `false` as engine: `image inspect` fails -> image missing
    let e = Engine { kind: Kind::Podman, bin: PathBuf::from("false") };
    let m = ensure_image(&e, "revised-sdk:0.0.0").unwrap_err();
    assert!(m.contains("revised-sdk setup"), "{m}");
}
