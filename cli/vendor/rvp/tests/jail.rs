//! Package jail + justified overrides (Package Format §5 Target-location policy, 2026-11-12).
use rvp::manifest::Manifest;
use rvp::pack::unpack_mem;
use rvp::stock::{check_stock, Stock};
use rvp::ustar::{ustar_write, UstarEntry};
use serde_json::{json, Value};

fn base(files: Value) -> Value {
    json!({
        "formatVersion": 1, "name": "demo", "version": "1.0.0", "author": "t",
        "targetCMU": "74.00.324A", "category": "c", "description": "d",
        "minDaemon": "1.0.0", "files": files
    })
}

/// Core holding the manifest plus a one-byte member for every `payload` it lists.
fn core(m: &Value) -> Vec<u8> {
    let mut members = vec![UstarEntry {
        name: "manifest.json".into(),
        data: serde_json::to_vec(m).unwrap(),
        is_dir: false,
    }];
    let mut ps: Vec<String> = m["files"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|f| f["payload"].as_str().map(String::from))
        .collect();
    ps.sort();
    for p in ps {
        members.push(UstarEntry {
            name: p,
            data: b"x".to_vec(),
            is_dir: false,
        });
    }
    rvp::pack::gzip(&ustar_write(&members).unwrap()).unwrap()
}

fn new_rel(live: &str) -> Value {
    json!({"live": live, "payload": format!("payload/{live}"), "mode": "0644",
           "owner": "1000:1000", "type": "new", "apply": "whole-file"})
}

fn modify(live: &str, reason: Option<&str>) -> Value {
    let mut f = json!({"live": live, "payload": "payload/m", "mode": "0755",
                       "owner": "1000:1000", "type": "modify", "apply": "3way"});
    if let Some(r) = reason {
        f["reason"] = r.into();
    }
    f
}

fn link(live: &str, target: &str) -> Value {
    json!({"live": live, "target": target, "mode": "0777", "owner": "1000:1000",
           "type": "link", "apply": "whole-file", "reason": "stock reader path"})
}

fn ok(files: Value) -> Manifest {
    unpack_mem(&core(&base(files))).unwrap().0
}

fn rejects(files: Value, needle: &str) {
    let e = unpack_mem(&core(&base(files))).unwrap_err().to_string();
    assert!(e.contains(needle), "want {needle:?} in: {e}");
}

#[test]
fn test_relative_new_is_jailed_and_needs_no_reason() {
    ok(json!([new_rel("www/index.html")]));
}

#[test]
fn test_relative_live_must_be_clean() {
    for bad in ["../etc/x", "www/../x", "./x", "www//x"] {
        let mut f = new_rel("www/x");
        f["live"] = bad.into();
        rejects(json!([f]), "live");
    }
}

#[test]
fn test_relative_live_must_be_type_new() {
    let mut f = modify("www/x", Some("r"));
    f["live"] = "www/x".into();
    rejects(json!([f]), "jail");
}

#[test]
fn test_absolute_override_requires_reason() {
    rejects(
        json!([modify("/jci/scripts/start_normal_mode.sh", None)]),
        "reason",
    );
    rejects(
        json!([modify("/jci/scripts/start_normal_mode.sh", Some("  "))]),
        "reason",
    );
    ok(json!([modify(
        "/jci/scripts/start_normal_mode.sh",
        Some("hook boot")
    )]));
    let mut outward = new_rel("x");
    outward["live"] = "/jci/new_thing".into();
    rejects(json!([outward.clone()]), "reason");
    outward["reason"] = "needed before data_persist".into();
    ok(json!([outward]));
}

#[test]
fn test_forbidden_prefixes_rejected_even_with_reason() {
    for bad in [
        "/proc/x",
        "/sys/x",
        "/dev/x",
        "/tmp/x",
        "/run/x",
        "/data_persist/cmu-re/vanilla/index",
        "/data_persist/cmu-re/pkg/other/x",
        "/data_persist/cmu-re/pkg/demo/x",
    ] {
        rejects(json!([modify(bad, Some("because"))]), "forbidden");
    }
    // similar-looking names are not the forbidden trees
    let mut f = modify("/procfs", Some("r"));
    f["payload"] = "payload/n".into();
    ok(json!([modify("/data_persist/cmu-ref", Some("r")), f]));
}

#[test]
fn test_link_shape() {
    ok(json!([
        new_rel("www/index.html"),
        link("/jci/gui/apps/demo", "www")
    ]));
    ok(json!([
        new_rel("www/index.html"),
        link("/jci/a", "www/index.html")
    ]));
    // target must be jailed content of this package
    rejects(
        json!([new_rel("www/index.html"), link("/jci/a", "other")]),
        "target",
    );
    rejects(
        json!([new_rel("www/index.html"), link("/jci/a", "ww")]),
        "target",
    );
    rejects(
        json!([new_rel("www/index.html"), link("/jci/a", "../x")]),
        "target",
    );
    rejects(
        json!([new_rel("www/index.html"), link("/jci/a", "/abs")]),
        "target",
    );
    // a link has no payload; other types have no target
    let mut l = link("/jci/a", "www");
    l["payload"] = "payload/l".into();
    rejects(json!([new_rel("www/index.html"), l]), "payload");
    let mut n = new_rel("www/index.html");
    n["target"] = "www".into();
    rejects(json!([n]), "target");
    // a link always lives outside the jail
    rejects(
        json!([new_rel("www/index.html"), link("lnk", "www")]),
        "jail",
    );
}

#[test]
fn test_payload_still_required_for_non_link() {
    let mut f = new_rel("www/x");
    f.as_object_mut().unwrap().remove("payload");
    rejects(json!([f]), "payload");
}

const STOCK: &str = "d /jci\nd /jci/gui/apps\nf /jci/scripts/start_normal_mode.sh\n\
l /jci/alias\nf /jci/gui/framework/js/Language Codes.txt\n";

fn stock_err(files: Value) -> String {
    check_stock(&ok(files), &Stock::parse(STOCK))
        .unwrap_err()
        .to_string()
}

#[test]
fn test_stock_modify_must_be_stock_regular_file() {
    let s = Stock::parse(STOCK);
    check_stock(
        &ok(json!([modify(
            "/jci/scripts/start_normal_mode.sh",
            Some("r")
        )])),
        &s,
    )
    .unwrap();
    check_stock(
        &ok(json!([modify(
            "/jci/gui/framework/js/Language Codes.txt",
            Some("r")
        )])),
        &s,
    )
    .unwrap();
    assert!(stock_err(json!([modify("/jci/nope.sh", Some("r"))])).contains("stock"));
    assert!(stock_err(json!([modify("/jci/alias", Some("r"))])).contains("regular"));
    assert!(stock_err(json!([modify("/jci/gui/apps", Some("r"))])).contains("regular"));
}

#[test]
fn test_stock_outward_new_and_link_must_be_absent() {
    let s = Stock::parse(STOCK);
    let mut n = new_rel("x");
    n["live"] = "/jci/scripts/start_normal_mode.sh".into();
    n["reason"] = "r".into();
    assert!(stock_err(json!([n.clone()])).contains("modify"));
    n["live"] = "/jci/brand_new".into();
    check_stock(&ok(json!([n])), &s).unwrap();
    assert!(stock_err(json!([new_rel("w/i"), link("/jci/gui/apps", "w")])).contains("exists"));
    check_stock(
        &ok(json!([new_rel("w/i"), link("/jci/gui/apps/demo", "w")])),
        &s,
    )
    .unwrap();
    // jailed files never touch the stock check
    check_stock(
        &ok(json!([new_rel("jci/scripts/start_normal_mode.sh")])),
        &s,
    )
    .unwrap();
}

#[test]
fn test_shipped_stock_list_parses() {
    let s = Stock::parse(include_str!("../stock/74.00.324A.paths"));
    assert_eq!(s.kind("/jci/scripts/start_normal_mode.sh"), Some('f'));
    assert_eq!(s.kind("/jci/gui/apps"), Some('d'));
    assert_eq!(s.kind("/jci/nope"), None);
}
