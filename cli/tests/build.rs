use revised_sdk::{build::build, new::new_project};
use std::path::{Path, PathBuf};

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rsdk-b-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn build_template_app() {
    let base = tmp("tpl");
    let proj = new_project(&base, "app", "t1").unwrap();
    let rvp = build(&proj).unwrap();
    assert_eq!(rvp, proj.join("dist").join("t1-0.1.0.rvp"));
    let bytes = std::fs::read(&rvp).unwrap();
    assert_eq!(&bytes[..3], b"RVP");
    assert_eq!(bytes[4], 0, "unsigned: flags 0");
    let core = &bytes[7..]; // magic(3) cver(1) flags(1) hlen(2), no sig block
    let tar = rvp::pack::gunzip(core).unwrap();
    assert!(!tar.is_empty());
    let (m, entries) = rvp::pack::unpack_mem(core).unwrap();
    assert_eq!((m.name.as_str(), m.target_cmu.as_str()), ("t1", "74.00.324A"));
    let live: Vec<_> = m.files.iter().map(|f| f.live.as_str()).collect();
    assert!(live.contains(&"ui/hmi.json") && live.contains(&"ui/main.js"), "{live:?}");
    assert!(m.files.iter().all(|f| !f.live.starts_with('/')), "everything inside the jail");
    let hmi = entries.iter().find(|e| e.name == "payload/ui/hmi.json").unwrap();
    let v: serde_json::Value = serde_json::from_slice(&hmi.data).unwrap();
    assert_eq!(v["entries"][0]["id"], "t1");
    assert_eq!(v["entries"][0]["main"], "main.js");
}

#[test]
fn build_overwrites_handwritten_hmi_and_reports_missing_entry() {
    let base = tmp("hand");
    let proj = new_project(&base, "app", "t2").unwrap();
    std::fs::write(proj.join("ui/hmi.json"), "{\"bogus\":1}").unwrap();
    let rvp = build(&proj).unwrap();
    let bytes = std::fs::read(rvp).unwrap();
    let (_, entries) = rvp::pack::unpack_mem(&bytes[7..]).unwrap();
    let hmi = entries.iter().find(|e| e.name == "payload/ui/hmi.json").unwrap();
    assert!(String::from_utf8_lossy(&hmi.data).contains("apiVersion"));
    let y = std::fs::read_to_string(proj.join("mod.yaml")).unwrap().replace("ui/main.js", "ui/nope.js");
    std::fs::write(proj.join("mod.yaml"), y).unwrap();
    let f = build(&proj).unwrap_err();
    assert_eq!(f[0].rule, "entry-missing");
}

#[test]
fn new_refuses_nonempty() {
    let base = tmp("ne");
    std::fs::create_dir_all(base.join("t3")).unwrap();
    std::fs::write(base.join("t3/x"), "x").unwrap();
    let f = new_project(&base, "app", "t3").unwrap_err();
    assert_eq!(f[0].rule, "exists");
    // empty existing dir is fine
    std::fs::create_dir_all(base.join("t4")).unwrap();
    assert!(new_project(&base, "app", "t4").is_ok());
    // bad names / templates
    assert_eq!(new_project(&base, "app", "../x").unwrap_err()[0].rule, "name");
    assert_eq!(new_project(&base, "nope", "t5").unwrap_err()[0].rule, "template");
}

#[test]
fn new_substitutes_name_and_binary_survives() {
    let base = tmp("sub");
    let p = new_project(&base, "app", "mydash").unwrap();
    let y = std::fs::read_to_string(p.join("mod.yaml")).unwrap();
    assert!(y.starts_with("name: mydash\n"), "{y}");
    assert!(std::fs::read_to_string(p.join("ui/main.js")).unwrap().contains("register(\"mydash/mydash\""));
    assert_eq!(&std::fs::read(p.join("ui/icon.png")).unwrap()[1..4], b"PNG");
    assert_eq!(std::fs::read_to_string(p.join(".gitignore")).unwrap(), "dist/\n.revised/\n");
}

#[test]
fn cli_new_exit_codes() {
    let base = tmp("cli");
    let bin = env!("CARGO_BIN_EXE_revised-sdk");
    let run = |args: &[&str]| {
        std::process::Command::new(bin).arg("-C").arg(&base).args(args).output().unwrap()
    };
    assert_eq!(run(&["new", "app", "c1"]).status.code(), Some(0));
    let o = run(&["--json", "new", "app", "c1"]);
    assert_eq!(o.status.code(), Some(1));
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!((v["ok"].clone(), v["errors"][0]["rule"].clone()), (false.into(), "exists".into()));
    assert_eq!(run(&["new", "app"]).status.code(), Some(2));
    let o = std::process::Command::new(bin).arg("-C").arg(base.join("c1")).arg("build").output().unwrap();
    assert_eq!(o.status.code(), Some(0));
    assert!(base.join("c1/dist/c1-0.1.0.rvp").is_file());
}

#[test]
fn new_leading_digit_name_gives_valid_css() {
    // Package Format allows a leading digit (`1app`); CSS class names must not start with one.
    let p = new_project(&tmp("digit"), "app", "1app").unwrap();
    let css = std::fs::read_to_string(p.join("ui/style.css")).unwrap();
    let js = std::fs::read_to_string(p.join("ui/main.js")).unwrap();
    for sel in css.lines().filter_map(|l| l.trim().strip_prefix('.')) {
        assert!(sel.as_bytes()[0].is_ascii_alphabetic(), "invalid class selector: .{sel}");
    }
    assert!(css.contains(".app-1app-title") && js.contains("\"app-1app-title\""), "{css}\n{js}");
}

#[test]
fn unknown_command_exits_2_without_an_engine() {
    let o = std::process::Command::new(env!("CARGO_BIN_EXE_revised-sdk"))
        .args(["--json", "lnit"])
        .env("PATH", "")
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&o.stdout).contains("unknown command"));
}

#[test]
fn build_passes_config_through() {
    let base = tmp("cfg");
    let proj = new_project(&base, "app", "t9").unwrap();
    std::fs::write(proj.join("ui/settings.js.tpl"), "var S = { n: {{ n }} };\n").unwrap();
    let cfg = "config:\n  fields:\n    - { key: n, type: number, label: Rows, default: 3, min: 1, max: 9 }\n  outputs:\n    - { template: ui/settings.js.tpl, path: ui/settings.js }\n  apply: none\n";
    let y = std::fs::read_to_string(proj.join("mod.yaml")).unwrap();
    std::fs::write(proj.join("mod.yaml"), format!("{y}{cfg}")).unwrap();
    let bytes = std::fs::read(build(&proj).unwrap()).unwrap();
    let (m, _) = rvp::pack::unpack_mem(&bytes[7..]).unwrap();
    let c = m.config.expect("config in the manifest");
    assert_eq!((c.fields[0].key.as_str(), c.outputs[0].path.as_str()), ("n", "ui/settings.js"));
    // a bad block is refused by the package checks, not silently dropped
    std::fs::write(proj.join("mod.yaml"), format!("{y}{}", cfg.replace("default: 3", "default: 30"))).unwrap();
    let f = build(&proj).unwrap_err();
    assert!(f[0].msg.contains("config:"), "{:?}", f[0].msg);
}

fn set_rows(proj: &std::path::Path, rows: &str, background: bool) {
    let y = std::fs::read_to_string(proj.join("mod.yaml")).unwrap();
    let mut out: Vec<String> = y.lines().filter(|l| !l.trim_start().starts_with("background:"))
        .map(|l| if l.trim_start().starts_with("rows:") { format!("  rows: {rows}") } else { l.to_string() }).collect();
    let i = out.iter().position(|l| l.trim_start().starts_with("rows:")).unwrap();
    out.insert(i + 1, format!("  background: {background}"));
    std::fs::write(proj.join("mod.yaml"), out.join("\n") + "\n").unwrap();
}

#[test]
fn build_background_without_rows() {
    let proj = new_project(&tmp("bg"), "app", "t5").unwrap();
    set_rows(&proj, "[]", true);
    let bytes = std::fs::read(build(&proj).unwrap()).unwrap();
    let (_, entries) = rvp::pack::unpack_mem(&bytes[7..]).unwrap();
    let hmi = entries.iter().find(|e| e.name == "payload/ui/hmi.json").unwrap();
    let v: serde_json::Value = serde_json::from_slice(&hmi.data).unwrap();
    assert_eq!((v["entries"][0]["section"].as_str(), v["entries"][0]["listen"].as_bool()), (Some("none"), Some(true)));
}

#[test]
fn build_rejects_empty_rows_without_background() {
    let proj = new_project(&tmp("nobg"), "app", "t6").unwrap();
    set_rows(&proj, "[]", false);
    assert_eq!(build(&proj).unwrap_err()[0].rule, "rows");
}

// Stock paths and modes come from the published baseline lists; the bytes are our own fixtures.
const FOCUS: &str = "/jci/gui/apps/xmdata/controls/xmdataList2/images/ListFocus.png";
const CSS: &str = "/jci/gui/apps/amradio/css/amradioApp.css";

fn fixture(f: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/skin-changes/changes").join(f)
}

/// Copies the fixture PNG and diff into `proj` and lists them as the `changes:` of `yaml`.
fn add_changes(proj: &Path, yaml: &str) {
    std::fs::create_dir_all(proj.join("skin")).unwrap();
    std::fs::copy(fixture("focus.png"), proj.join("skin/focus.png")).unwrap();
    std::fs::copy(fixture("clean.css.diff"), proj.join("skin/am.css.diff")).unwrap();
    let chg = format!(
        "changes:\n  - live: {FOCUS}\n    file: skin/focus.png\n    apply: whole-file\n    reason: new focus bar\n  \
         - live: {CSS}\n    file: skin/am.css.diff\n    apply: three-way\n    reason: red accent\n"
    );
    std::fs::write(proj.join("mod.yaml"), yaml.replace("changes: []\n", &chg)).unwrap();
}

const SKIN: &str = "name: s1\nversion: 1.0.0\ntargetCMU: \"74.00.324A\"\ncategory: Skins\nchanges: []\n";

fn unpack(rvp: &Path) -> (rvp::manifest::Manifest, Vec<rvp::ustar::UstarEntry>) {
    rvp::pack::unpack_mem(&std::fs::read(rvp).unwrap()[7..]).unwrap()
}

fn modify_json(m: &rvp::manifest::Manifest) -> Vec<serde_json::Value> {
    let v = serde_json::to_value(m).unwrap();
    v["files"].as_array().unwrap().iter().filter(|f| f["type"] == "modify").cloned().collect()
}

#[test]
fn build_skin_packages_changes() {
    let proj = tmp("skin");
    add_changes(&proj, SKIN);
    let (m, entries) = unpack(&build(&proj).unwrap());
    assert_eq!((m.category.as_str(), m.description.as_str(), m.restart), ("Skins", "s1", true));
    let mods = modify_json(&m);
    assert_eq!(mods.len(), m.files.len(), "no app: only modify entries");
    let png = serde_json::json!({"live": FOCUS, "payload": format!("payload/stock{FOCUS}"), "mode": "0755",
        "owner": "1000:1000", "type": "modify", "apply": "whole-file", "reason": "new focus bar", "revert": {}});
    let css = serde_json::json!({"live": CSS, "payload": format!("payload/stock{CSS}"), "mode": "0755",
        "owner": "1000:1000", "type": "modify", "apply": "3way", "reason": "red accent", "revert": {}});
    assert_eq!(mods, vec![png, css]);
    assert!(!entries.iter().any(|e| e.name.starts_with("payload/ui/")), "no ui/ walk, no hmi.json");
    let data = |p: String| entries.iter().find(|e| e.name == p).unwrap().data.clone();
    assert_eq!(data(format!("payload/stock{FOCUS}")), std::fs::read(fixture("focus.png")).unwrap());
    assert_eq!(data(format!("payload/stock{CSS}")), std::fs::read(fixture("clean.css.diff")).unwrap());
}

#[test]
fn build_app_with_changes_has_both_kinds() {
    let proj = new_project(&tmp("appchg"), "app", "t7").unwrap();
    add_changes(&proj, &std::fs::read_to_string(proj.join("mod.yaml")).unwrap());
    let (m, entries) = unpack(&build(&proj).unwrap());
    assert_eq!((m.category.as_str(), m.description.as_str(), m.restart), ("Apps", "t7", true));
    assert_eq!(modify_json(&m).len(), 2);
    assert!(m.files.iter().any(|f| f.live == "ui/main.js" && f.r#type == rvp::manifest::FileType::New));
    assert!(entries.iter().any(|e| e.name == "payload/ui/hmi.json"));
}

#[test]
fn build_app_without_changes_needs_no_restart() {
    let proj = new_project(&tmp("norestart"), "app", "t8").unwrap();
    assert!(!unpack(&build(&proj).unwrap()).0.restart);
}

#[test]
fn build_refuses_a_change_outside_the_baseline() {
    let proj = tmp("nostock");
    add_changes(&proj, &SKIN.replace("s1", "s2"));
    let y = std::fs::read_to_string(proj.join("mod.yaml")).unwrap().replace(CSS, "/jci/gui/apps/amradio/css/none.css");
    std::fs::write(proj.join("mod.yaml"), y).unwrap();
    let f = build(&proj).unwrap_err();
    assert_eq!(f[0].rule, "stock-path", "{:?}", f[0].msg);
    assert!(!proj.join("dist").exists());
}
