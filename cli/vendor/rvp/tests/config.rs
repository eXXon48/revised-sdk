//! Package config (manifest `config`): schema, values, merge, template render.
mod common;

use rvp::config::Values;
use rvp::manifest::Manifest;
use rvp::pack::{gzip, pack_dir, unpack_mem};
use rvp::ustar::{ustar_write, UstarEntry};
use serde_json::{json, Value};

const TPL: &str = "skin/accent.css.tpl";

fn example() -> Value {
    json!({
        "fields": [
            { "key": "accent", "type": "select", "label": "Accent colour", "default": "#FF2A30",
              "options": [ { "label": "Red", "value": "#FF2A30" }, { "label": "Green", "value": "#2ECC71" },
                           { "label": "Blue", "value": "#2E86FF" } ] },
            { "key": "size", "type": "range", "label": "Font size", "default": 16, "min": 12, "max": 24, "step": 2 },
            { "key": "count", "type": "number", "label": "Rows", "default": 3, "min": 1, "max": 9 },
            { "key": "clock", "type": "switch", "label": "Show clock", "default": true, "on": "1", "off": "0" }
        ],
        "outputs": [ { "template": TPL, "path": "skin/accent.css" } ],
        "apply": "none"
    })
}

fn manifest(config: Option<Value>) -> Value {
    let mut m = json!({
        "formatVersion": 1, "name": "demo", "version": "1.0.0", "author": "t",
        "targetCMU": "74.00.324A", "category": "c", "description": "d", "minDaemon": "1.0.0",
        "files": [
            {"live": TPL, "payload": format!("payload/{TPL}"), "mode": "0644", "owner": "1000:1000", "type": "new", "apply": "whole-file"},
            {"live": "www/app.js", "payload": "payload/www/app.js", "mode": "0644", "owner": "1000:1000", "type": "new", "apply": "whole-file"}
        ]
    });
    if let Some(c) = config {
        m["config"] = c;
    }
    m
}

fn core(m: &Value) -> Vec<u8> {
    let mut members = vec![UstarEntry { name: "manifest.json".into(), data: serde_json::to_vec(m).unwrap(), is_dir: false }];
    for p in [format!("payload/{TPL}"), "payload/www/app.js".to_string()] {
        members.push(UstarEntry { name: p, data: b"x".to_vec(), is_dir: false });
    }
    gzip(&ustar_write(&members).unwrap()).unwrap()
}

fn parsed(c: Value) -> Result<Manifest, String> {
    unpack_mem(&core(&manifest(Some(c)))).map(|r| r.0).map_err(|e| e.to_string())
}

fn cfg() -> rvp::config::Config {
    parsed(example()).unwrap().config.unwrap()
}

fn vals(v: Value) -> Values {
    serde_json::from_value(v).unwrap()
}

#[test]
fn schema_example_roundtrips() {
    let m = parsed(example()).unwrap();
    let canon = m.to_canonical_json().unwrap();
    let m2 = Manifest::parse(&canon).unwrap();
    assert_eq!(m, m2);
    assert_eq!(m2.to_canonical_json().unwrap(), canon);
    let plain = unpack_mem(&core(&manifest(None))).unwrap().0;
    assert!(plain.config.is_none());
    assert!(!String::from_utf8(plain.to_canonical_json().unwrap()).unwrap().contains("\"config\""));
}

#[test]
fn bad_schemas_rejected() {
    let cases: Vec<(&str, Box<dyn Fn(&mut Value)>)> = vec![
        ("key Accent", Box::new(|c| c["fields"][0]["key"] = "Accent".into())),
        ("dup key", Box::new(|c| c["fields"][1]["key"] = "accent".into())),
        ("blank label", Box::new(|c| c["fields"][0]["label"] = " ".into())),
        ("one option", Box::new(|c| c["fields"][0]["options"] = json!([{"label": "Red", "value": "#FF2A30"}]))),
        ("dup option value", Box::new(|c| c["fields"][0]["options"][1]["value"] = "#FF2A30".into())),
        ("select default", Box::new(|c| c["fields"][0]["default"] = "red".into())),
        ("range step grid", Box::new(|c| c["fields"][1]["step"] = 5.into())),
        ("range step 0", Box::new(|c| c["fields"][1]["step"] = 0.into())),
        ("range min>=max", Box::new(|c| c["fields"][1]["max"] = 12.into())),
        ("range default off grid", Box::new(|c| c["fields"][1]["default"] = 17.into())),
        ("number min>max", Box::new(|c| { c["fields"][2]["min"] = 5.into(); c["fields"][2]["max"] = 4.into(); })),
        ("number default", Box::new(|c| c["fields"][2]["default"] = 10.into())),
        ("template not a payload", Box::new(|c| c["outputs"][0]["template"] = "skin/other.tpl".into())),
        ("path ..", Box::new(|c| c["outputs"][0]["path"] = "../x".into())),
        ("path absolute", Box::new(|c| c["outputs"][0]["path"] = "/etc/x".into())),
        ("path empty part", Box::new(|c| c["outputs"][0]["path"] = "skin//x".into())),
        ("path is a payload live", Box::new(|c| c["outputs"][0]["path"] = "www/app.js".into())),
        ("two outputs same path", Box::new(|c| c["outputs"] = json!([{"template": TPL, "path": "a.css"}, {"template": TPL, "path": "a.css"}]))),
        ("no fields", Box::new(|c| c["fields"] = json!([]))),
        ("bad apply", Box::new(|c| c["apply"] = "restart".into())),
        ("output is a payload dir", Box::new(|c| c["outputs"][0]["path"] = "skin".into())),
        ("output under a payload file", Box::new(|c| c["outputs"][0]["path"] = "www/app.js/x".into())),
        ("output inside another output", Box::new(|c| c["outputs"] = json!([{"template": TPL, "path": "a"}, {"template": TPL, "path": "a/b"}]))),
        ("output temp name taken", Box::new(|c| c["outputs"] = json!([{"template": TPL, "path": "a"}, {"template": TPL, "path": "a.cfgtmp"}]))),
    ];
    for (what, f) in cases {
        let mut c = example();
        f(&mut c);
        assert!(parsed(c).is_err(), "accepted: {what}");
    }
}

#[test]
fn render_values() {
    let c = cfg();
    let v = vals(json!({"accent": "#2ECC71", "size": 18, "count": 3, "clock": false}));
    assert_eq!(c.render("a{{accent}}b{{ size }}c{{clock}}", &v).unwrap(), "a#2ECC71b18c0");
    assert!(c.render("{{nope}}", &v).is_err());
    assert!(c.render("x{{accent", &v).is_err());
    let on = vals(json!({"accent": "#2ECC71", "size": 18, "count": 3, "clock": true}));
    assert_eq!(c.render("{{clock}}", &on).unwrap(), "1");
}

#[test]
fn switch_on_off_default_strings() {
    let mut e = example();
    e["fields"][3] = json!({"key": "clock", "type": "switch", "label": "Clock", "default": false});
    let c = parsed(e).unwrap().config.unwrap();
    assert_eq!(c.render("{{clock}}", &c.defaults()).unwrap(), "false");
}

#[test]
fn check_values_cases() {
    let c = cfg();
    let d = c.defaults();
    assert_eq!(d, vals(json!({"accent": "#FF2A30", "size": 16, "count": 3, "clock": true})));
    c.check_values(&d).unwrap();
    let bad = |k: &str, x: Value| {
        let mut v = d.clone();
        v.insert(k.into(), x);
        v
    };
    for v in [
        bad("size", json!(17)),
        bad("size", json!(26)),
        bad("size", json!(16.0)),
        bad("accent", json!("#000")),
        bad("clock", json!("1")),
        bad("count", json!(0)),
        bad("extra", json!(1)),
    ] {
        assert!(c.check_values(&v).is_err(), "accepted {v:?}");
    }
    let mut missing = d.clone();
    missing.remove("count");
    assert!(c.check_values(&missing).is_err());
}

#[test]
fn merge_keeps_valid_drops_rest() {
    let c = cfg();
    let old = vals(json!({"accent": "#2E86FF", "size": 30, "gone": 1}));
    assert_eq!(c.merge(&old), vals(json!({"accent": "#2E86FF", "size": 16, "count": 3, "clock": true})));
}

#[test]
fn pack_rejects_unknown_placeholder() {
    let d = common::tmp("cfg-pack");
    let write = |p: &str, s: &str| {
        let f = d.join(p);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, s).unwrap();
    };
    write("manifest.json", &manifest(Some(example())).to_string());
    write("payload/www/app.js", "x");
    write(&format!("payload/{TPL}"), ".a{color:{{ accent }}}");
    pack_dir(&d).unwrap();
    write(&format!("payload/{TPL}"), ".a{color:{{ nope }}}");
    let e = pack_dir(&d).unwrap_err().to_string();
    assert!(e.contains("nope"), "{e}");
}
