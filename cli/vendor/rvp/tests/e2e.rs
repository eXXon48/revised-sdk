//! End-to-end CI flow (Signing & Pipeline §5): emit core -> minisign -> wrap -> verify.
//! Needs `minisign` on PATH (present in the rvp-build image).
mod common;
use common::{fixture, tmp};
use rvp::container::container_read;
use rvp::hash::blake2b512_hex;
use std::path::Path;
use std::process::{Command, Output};

const PACK: &str = env!("CARGO_BIN_EXE_rvp-pack");
const VERIFY: &str = env!("CARGO_BIN_EXE_rvp-verify");

fn run(bin: &str, args: &[&Path]) -> Output {
    Command::new(bin).args(args).output().unwrap()
}

fn ok(o: Output) -> String {
    assert!(
        o.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8(o.stdout).unwrap()
}

fn minisign(args: &[&str]) {
    let o = Command::new("minisign").args(args).output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}

#[test]
fn test_e2e_ci_flow() {
    let d = tmp("e2e");
    let p = |n: &str| d.join(n);
    let s = |n: &str| p(n).to_str().unwrap().to_string();
    let pkg = fixture("pkg");

    // build job: emit the core, print the hash the catalog pins
    let out = ok(run(PACK, &[Path::new("--emit-core"), &pkg, &p("out.core")]));
    let core = std::fs::read(p("out.core")).unwrap();
    assert_eq!(out.trim(), blake2b512_hex(&core));

    // sign job: fresh throwaway key, prehashed signature over the exact core
    minisign(&["-G", "-W", "-p", &s("test.pub"), "-s", &s("test.key")]);
    minisign(&[
        "-S",
        "-s",
        &s("test.key"),
        "-m",
        &s("out.core"),
        "-x",
        &s("out.core.minisig"),
        "-t",
        "revised-approved 2026-11-12T00:00:00Z",
    ]);

    // wrap the exact signed core, then verify
    ok(run(
        PACK,
        &[
            Path::new("--from-core"),
            &p("out.core"),
            &p("out.rvp"),
            Path::new("--sig"),
            &p("out.core.minisig"),
        ],
    ));
    ok(run(VERIFY, &[&p("out.rvp"), &p("test.pub")]));

    // any flipped byte (header, sig-block or core) fails verification
    let rvp = std::fs::read(p("out.rvp")).unwrap();
    for i in [4, 20, rvp.len() - 600] {
        let mut bad = rvp.clone();
        bad[i] ^= 1;
        std::fs::write(p("bad.rvp"), &bad).unwrap();
        let o = run(VERIFY, &[&p("bad.rvp"), &p("test.pub")]);
        assert!(!o.status.success(), "flip at {i} verified");
        assert!(!o.stderr.is_empty());
    }

    // one-shot pack == emit-core + from-core (determinism)
    ok(run(
        PACK,
        &[
            &pkg,
            &p("oneshot.rvp"),
            Path::new("--sig"),
            &p("out.core.minisig"),
        ],
    ));
    let oneshot = std::fs::read(p("oneshot.rvp")).unwrap();
    assert_eq!(container_read(&oneshot).unwrap().core, core);
    assert_eq!(oneshot, rvp);

    // unsigned package never verifies
    ok(run(PACK, &[&pkg, &p("bare.rvp")]));
    let o = run(VERIFY, &[&p("bare.rvp"), &p("test.pub")]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("unsigned"));
}

#[test]
fn test_pack_rejects_override_against_stock() {
    // modify of a path stock 74.00.324A does not have: refused, nothing written
    let d = common::pkg_copy("stock");
    let mpath = d.join("manifest.json");
    let m = std::fs::read_to_string(&mpath).unwrap().replace(
        "\"live\": \"/jci/scripts/start_normal_mode.sh\"",
        "\"live\": \"/jci/scripts/no_such_script.sh\"",
    );
    std::fs::write(&mpath, m).unwrap();
    let out = d.join("out.core");
    for args in [vec![Path::new("--emit-core"), &d, &out], vec![&d, &out]] {
        let o = run(PACK, &args);
        assert!(!o.status.success());
        assert!(String::from_utf8_lossy(&o.stderr).contains("not a stock file"));
        assert!(!out.exists());
    }
}

#[test]
fn test_cli_usage_errors() {
    let o = Command::new(PACK).output().unwrap();
    assert!(!o.status.success());
    let o = Command::new(VERIFY).arg("only-one-arg").output().unwrap();
    assert!(!o.status.success());
}

#[test]
fn pack_with_extra_stock() {
    let d = common::pkg_copy("extra");
    let mpath = d.join("manifest.json");
    let m = std::fs::read_to_string(&mpath)
        .unwrap()
        .replace(
            "\"live\": \"/jci/scripts/start_normal_mode.sh\"",
            "\"live\": \"/data/cmu-re-test/stock/test.txt\"",
        )
        .replacen("\"whole-file\"", "\"3way\"", 1);
    std::fs::write(&mpath, m).unwrap();
    let out = d.join("out.core");
    let o = run(PACK, &[Path::new("--emit-core"), &d, &out]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("not a stock file"), "{}", String::from_utf8_lossy(&o.stderr));
    let extra = d.join("extra.paths");
    std::fs::write(&extra, "f /data/cmu-re-test/stock/test.txt\n").unwrap();
    let x = Path::new("--extra-stock");
    // flag at the end, at the front, and in the src/out form
    for args in [
        vec![Path::new("--emit-core"), &d, &out, x, &extra],
        vec![x, &extra, Path::new("--emit-core"), &d, &out],
        vec![&d, &d.join("o.rvp"), x, &extra],
    ] {
        ok(run(PACK, &args));
    }
    assert!(out.exists());
}
