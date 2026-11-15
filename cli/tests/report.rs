use revised_sdk::report::*;
use serde_json::{json, Value};

#[test]
fn json_report_shape() {
    let r = Report::new("check", json!({}), vec![Finding::error("r1", "m").at("a.js", 1, 2)]);
    let v: Value = serde_json::to_value(&r).unwrap();
    let mut keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["command", "data", "errors", "ok"]);
    let e = &v["errors"][0];
    assert_eq!(e["level"], "error");
    assert_eq!(e["line"], 1);
    let w = serde_json::to_value(Finding::warning("w", "m")).unwrap();
    assert!(w["file"].is_null() && w["line"].is_null() && w["col"].is_null());
    assert_eq!(w["level"], "warning");
}

#[test]
fn exit_codes() {
    assert_eq!(Report::new("c", json!({}), vec![]).emit(true), 0);
    assert_eq!(Report::new("c", json!({}), vec![Finding::warning("w", "m")]).emit(false), 0);
    assert_eq!(Report::new("c", json!({}), vec![Finding::error("e", "m")]).emit(true), 1);
    assert_eq!(env_error("c", "boom"), 2);
}
