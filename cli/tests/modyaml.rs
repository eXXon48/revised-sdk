use revised_sdk::build::hmi_entries;
use revised_sdk::modyaml::{parse, ChangeApply};
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

const SKIN: &str = "changes:\n  - live: /jci/gui/index.html\n    file: stock/index.html\n    apply: whole-file\n    reason: new skin\n";

#[test]
fn skin_only_parses() {
    let m = parse(&yaml(SKIN)).unwrap();
    assert!(m.app.is_none());
    assert_eq!(m.changes.len(), 1);
    assert_eq!((m.changes[0].live.as_str(), m.changes[0].file.as_str()), ("/jci/gui/index.html", "stock/index.html"));
    assert_eq!(m.changes[0].apply, ChangeApply::WholeFile);
    assert_eq!(m.changes[0].reason, "new skin");
    assert_eq!(m.category(), "System mods");
}

#[test]
fn category_default_and_explicit() {
    let m = parse(&yaml("app:\n  title: T\n  entry: ui/main.js\n  rows: [apps]\n")).unwrap();
    assert_eq!(m.category(), "Apps");
    let m = parse(&yaml(&format!("category: Skins\n{SKIN}"))).unwrap();
    assert_eq!(m.category(), "Skins");
    let m = parse(&yaml(&format!("category: System mods\n{SKIN}"))).unwrap();
    assert_eq!(m.category(), "System mods");
    assert_eq!(rules(&yaml(&format!("category: Themes\n{SKIN}")))[0].0, "category");
}

#[test]
fn no_app_and_no_changes() {
    assert_eq!(rules(&yaml(""))[0].0, "no-app");
    assert_eq!(rules(&yaml("changes: []\n"))[0].0, "no-app");
}

#[test]
fn change_shape_rules() {
    let one = |live: &str, file: &str, reason: &str| {
        let y = format!("changes:\n  - live: \"{live}\"\n    file: '{file}'\n    apply: three-way\n    reason: \"{reason}\"\n");
        parse(&yaml(&y)).map(|m| m.changes[0].apply).map_err(|e| e[0].rule.clone())
    };
    assert_eq!(one("/jci/x", "stock/x", "r"), Ok(ChangeApply::ThreeWay));
    assert_eq!(one("jci/x", "stock/x", "r"), Err("change-live".into()));
    assert_eq!(one("/jci/x", "stock/a..b", "r"), Ok(ChangeApply::ThreeWay));
    for f in ["ui/x", "ui", "a/../b", "/abs", "", "./ui/x", "ui\\x", "ui/./x", "a//b", "a/"] {
        assert_eq!(one("/jci/x", f, "r"), Err("change-file".into()), "{f}");
    }
    assert_eq!(one("/jci/x", "stock/x", " "), Err("change-reason".into()));
}

#[test]
fn two_changes_to_one_live_are_refused() {
    let ch = |live: &str, file: &str, apply: &str| {
        format!("  - live: {live}\n    file: {file}\n    apply: {apply}\n    reason: r\n")
    };
    for first in ["three-way", "whole-file"] {
        let y = yaml(&format!(
            "changes:\n{}{}{}",
            ch("/jci/a.css", "skin/a.diff", first),
            ch("/jci/b.css", "skin/b.diff", "three-way"),
            ch("/jci/a.css", "skin/c.diff", "three-way")
        ));
        let r = rules(&y);
        assert_eq!(r.len(), 1, "{r:?}");
        assert_eq!(r[0].0, "change-live");
        assert!(r[0].1.contains("changes[0]") && r[0].1.contains("changes[2]") && r[0].1.contains("/jci/a.css"), "{}", r[0].1);
    }
}

#[test]
fn bad_apply_names_allowed_values() {
    let r = rules(&yaml("changes:\n  - live: /a\n    file: b\n    apply: 3way\n    reason: r\n"));
    assert_eq!(r[0].0, "mod-yaml");
    assert!(r[0].1.contains("whole-file") && r[0].1.contains("three-way"), "{}", r[0].1);
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

#[test]
fn bad_name_and_version_are_rejected() {
    let bad = |n: &str, v: &str, rule: &str| {
        let t = format!("name: {n}\nversion: \"{v}\"\ntargetCMU: \"74.00.324A\"\n");
        let e = parse(&t).unwrap_err();
        assert!(e.iter().any(|f| f.rule == rule), "{n} {v}: {e:?}");
    };
    for n in ["a.b", "a_b", "../x", "A", "-a", ""] {
        bad(n, "1.0.0", "name");
    }
    for v in ["1/../../x", "1.0 beta", "1;rm", "1\\\"2", "$(x)", "..", "1..2", ".1", "-1", "", "1/2"] {
        bad("ok", v, "version");
    }
    let t = "name: ok\nversion: 1.0.0-rc.1+b2\ntargetCMU: \"74.00.324A\"\napp:\n  title: T\n  entry: ui/main.js\n  rows: [apps]\n";
    assert!(parse(t).is_ok());
}

#[test]
fn hmi_background_no_rows() {
    let m = parse(&yaml("app:\n  title: T\n  entry: ui/main.js\n  rows: []\n  background: true\n")).unwrap();
    assert_eq!(
        hmi_entries(&m),
        json!({"apiVersion": 1, "entries": [
            {"id": "mydash", "section": "none", "label": "T", "main": "main.js", "listen": true}
        ]})
    );
}
