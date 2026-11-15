use revised_sdk::{build::build, new::new_project};
use std::path::PathBuf;

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
    std::fs::remove_file(proj.join("ui/main.js")).unwrap();
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
