mod common;
use common::pkg_copy;
use rvp::deps::{cmp_version, parse_rels, satisfies, Rel, RelOp};
use rvp::pack::pack_dir;
use rvp::RvpError;
use std::cmp::Ordering::*;

fn rel(s: &str) -> Rel {
    parse_rels(s).unwrap().remove(0).remove(0)
}

#[test]
fn parse_and_or() {
    let r = parse_rels("a (>= 1.0), b | c").unwrap();
    assert_eq!(r.len(), 2);
    assert_eq!(
        r[0],
        vec![Rel {
            name: "a".into(),
            ver: Some((RelOp::Ge, "1.0".into()))
        }]
    );
    let names: Vec<&str> = r[1].iter().map(|x| x.name.as_str()).collect();
    assert_eq!(names, ["b", "c"]);
}

#[test]
fn parse_empty() {
    assert!(parse_rels("").unwrap().is_empty());
    assert!(parse_rels("  ").unwrap().is_empty());
}

#[test]
fn parse_bad() {
    for bad in ["a (=> 1)", "a (>= )", "a,,b", "../x"] {
        assert!(
            matches!(parse_rels(bad), Err(RvpError::Manifest(_))),
            "{bad}"
        );
    }
}

#[test]
fn dpkg_order() {
    let chain = ["1.0~rc1", "1.0", "1.0-1", "1.0.1", "1:0.1"];
    for w in chain.windows(2) {
        assert_eq!(cmp_version(w[0], w[1]), Less, "{} < {}", w[0], w[1]);
        assert_eq!(cmp_version(w[1], w[0]), Greater);
    }
    assert_eq!(cmp_version("1.a", "1.1"), Greater);
    assert_eq!(cmp_version("1.0.1", "1.0.rc1"), Less);
    assert_eq!(cmp_version("1.0~1", "1.0"), Less);
    assert_eq!(cmp_version("1~", "1.0"), Less);
    assert_eq!(cmp_version("1.10", "1.9"), Greater);
    assert_eq!(cmp_version("1.0a", "1.0"), Greater);
    assert_eq!(cmp_version("1.0", "1.0"), Equal);
}

#[test]
fn satisfies_ops() {
    let lt = rel("b (<< 2.0)");
    assert!(satisfies(&lt, "b", "1.9"));
    assert!(!satisfies(&lt, "b", "2.0"));
    assert!(!satisfies(&lt, "c", "1.9"));
    assert!(satisfies(&rel("b (= 1.0)"), "b", "1.0"));
    assert!(satisfies(&rel("b"), "b", "9:9"));
}

#[test]
fn manifest_conflicts_rejects_or() {
    let d = pkg_copy("conflicts-or");
    let p = d.join("manifest.json");
    let mut m: serde_json::Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    m["conflicts"] = "a | b".into();
    std::fs::write(&p, serde_json::to_vec(&m).unwrap()).unwrap();
    assert!(pack_dir(&d).is_err());
}
