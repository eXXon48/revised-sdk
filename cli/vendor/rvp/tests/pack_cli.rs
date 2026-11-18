//! rvp-pack CLI forms.
mod common;
use common::{fixture, tmp};
use std::path::Path;
use std::process::Command;

const PACK: &str = env!("CARGO_BIN_EXE_rvp-pack");

fn pack(args: &[&Path]) -> String {
    let o = Command::new(PACK).args(args).output().unwrap();
    assert!(
        o.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8(o.stdout).unwrap()
}

#[test]
fn extract_core_roundtrip() {
    let d = tmp("pack_cli");
    let p = |n: &str| d.join(n);
    let h = pack(&[Path::new("--emit-core"), &fixture("pkg"), &p("a.core")]);
    pack(&[Path::new("--from-core"), &p("a.core"), &p("x.rvp")]);
    let h2 = pack(&[Path::new("--extract-core"), &p("x.rvp"), &p("b.core")]);
    assert_eq!(h, h2);
    assert_eq!(
        std::fs::read(p("a.core")).unwrap(),
        std::fs::read(p("b.core")).unwrap()
    );
}
