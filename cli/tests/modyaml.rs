use revised_sdk::build::hmi_entries;
use revised_sdk::modyaml::parse;
use serde_json::json;

fn yaml(extra: &str) -> String {
    format!("name: mydash\nversion: 0.1.0\ntargetCMU: \"74.00.324A\"\n{extra}")
}

#[test]
fn hmi_one_row() {
    let m = parse(&yaml("app:\n  title: My Dash\n  entry: ui/main.js\n  rows: [apps]\n")).unwrap();
    assert_eq!(
        hmi_entries(&m),
        json!({"apiVersion": 1, "entries": [
            {"id": "mydash", "section": "applications", "label": "My Dash", "main": "main.js", "listen": false}
        ]})
    );
}

#[test]
fn hmi_two_rows_ids() {
    let m = parse(&yaml("app:\n  title: T\n  entry: ui/main.js\n  rows: [apps, entertainment]\n")).unwrap();
    let v = hmi_entries(&m);
    let e = v["entries"].as_array().unwrap();
    assert_eq!(e.len(), 2);
    assert_eq!((e[0]["id"].as_str(), e[0]["section"].as_str()), (Some("mydash-apps"), Some("applications")));
    assert_eq!((e[1]["id"].as_str(), e[1]["section"].as_str()), (Some("mydash-entertainment"), Some("entertainment")));
}

#[test]
fn hmi_listen_from_background() {
    let m = parse(&yaml("app:\n  title: T\n  entry: ui/main.js\n  rows: [apps]\n  background: true\n")).unwrap();
    assert_eq!(hmi_entries(&m)["entries"][0]["listen"], json!(true));
}

fn rules(y: &str) -> Vec<(String, String)> {
    parse(y).unwrap_err().into_iter().map(|f| (f.rule, f.msg)).collect()
}

#[test]
fn refuse_target_cmu() {
    let r = rules("name: a\nversion: 1.0.0\ntargetCMU: \"75.00.000A\"\n");
    assert_eq!(r[0].0, "target-cmu");
    assert!(r[0].1.contains("74.00.324A"), "{}", r[0].1);
}

#[test]
fn refuse_changes_phase2() {
    let r = rules(&yaml("changes:\n  - new-file: www/index.html\n"));
    assert_eq!(r[0].0, "changes-phase2");
    assert!(parse(&yaml("changes: []\n")).is_ok());
}

#[test]
fn refuse_native_phase2() {
    let r = rules(&yaml("native:\n  crate: native/\n"));
    assert_eq!(r[0].0, "native-phase2");
}

#[test]
fn bad_yaml_is_finding_with_location() {
    let f = parse("name: [\n").unwrap_err();
    assert_eq!(f[0].rule, "mod-yaml");
    assert!(f[0].line.is_some());
}
