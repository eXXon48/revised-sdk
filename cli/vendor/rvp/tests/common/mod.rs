// Shared test helpers (included via `mod common;`).
#![allow(dead_code)]
use std::path::{Path, PathBuf};

pub fn fixture(p: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(p)
}

/// A fresh empty temp dir unique to `tag`.
pub fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rvp-test-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Copy the fixture package tree into a temp dir so a test can mutate it.
pub fn pkg_copy(tag: &str) -> PathBuf {
    fn cp(src: &Path, dst: &Path) {
        std::fs::create_dir_all(dst).unwrap();
        for e in std::fs::read_dir(src).unwrap() {
            let e = e.unwrap();
            let to = dst.join(e.file_name());
            if e.file_type().unwrap().is_dir() {
                cp(&e.path(), &to);
            } else {
                std::fs::copy(e.path(), to).unwrap();
            }
        }
    }
    let d = tmp(tag);
    cp(&fixture("pkg"), &d);
    d
}
