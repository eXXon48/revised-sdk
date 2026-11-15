use revised_sdk::lint::run;
use revised_sdk::new::new_project;
use revised_sdk::report::{Finding, Level};
use std::path::{Path, PathBuf};

fn proj(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rsdk-l-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    new_project(&d, "app", "m").unwrap()
}

fn put(p: &Path, rel: &str, body: impl AsRef<[u8]>) {
    std::fs::write(p.join(rel), body).unwrap();
}

/// main.js = valid register call + `extra`.
fn js(name: &str, extra: &str) -> (PathBuf, Vec<Finding>) {
    let p = proj(name);
    put(&p, "ui/main.js", format!("RevisedHMI.register(\"m/m\", {{}});\n{extra}\n"));
    let f = run(&p);
    (p, f)
}

fn rules<'a>(f: &'a [Finding], rule: &str) -> Vec<&'a Finding> {
    f.iter().filter(|x| x.rule == rule).collect()
}

#[test]
fn clean_template_has_no_findings() {
    let p = proj("clean");
    assert!(run(&p).is_empty(), "{:?}", run(&p));
}

#[test]
fn es5_tokens_flagged_with_position() {
    for (src, col) in [
        ("let a = 1;", 1),
        ("const a = 1;", 1),
        ("var f = (x) => x;", 13),
        ("class A {}", 1),
        ("var s = `t`;", 9),
        ("f(...a);", 3),
        ("for (var x of xs) {}", 12),
        ("async function f() {}", 1),
        ("function f() { await g(); }", 16),
    ] {
        let (_, f) = js("es5", src);
        let h = rules(&f, "es5-syntax");
        assert_eq!(h.len(), 1, "{src}: {f:?}");
        assert_eq!((h[0].line, h[0].col), (Some(2), Some(col)), "{src}");
        assert_eq!(h[0].file.as_deref(), Some("ui/main.js"));
        assert_eq!(h[0].level, Level::Error);
    }
}

#[test]
fn es5_tokens_inside_strings_comments_regex_props_are_fine() {
    let src = r#"
var a = "let const class => ... async await `";
var b = 'x \' let';
// let const => class
/* for (x of y) { let } */
var c = /let|const|=>|`/g;
var d = x.class + x.const + x.async + o.of;
var e = { class: 1, let: 2 };
var g = a / b / c;
for (var i = 0; i < n; i++) { f(of); }
var classes = 1, letter = 2, asyncs = 3, constant = 4, awaiting = 5;
var h = [1].map(function (v) { return /=>/.test(v); });
"#;
    let (_, f) = js("neg", src);
    assert!(f.is_empty(), "{f:?}");
}

#[test]
fn minified_one_line() {
    // multibyte char before the hit: col counts characters, not bytes
    let (_, f) = js("min", "var a=\"é\";var b=1;let c=2;");
    let h = rules(&f, "es5-syntax");
    assert_eq!(h.len(), 1, "{f:?}");
    assert_eq!((h[0].line, h[0].col), (Some(2), Some(19)));

    let p = proj("min1");
    put(&p, "ui/main.js", "RevisedHMI.register(\"m/m\",{});let z=1;");
    let h = run(&p);
    assert_eq!((h[0].rule.as_str(), h[0].line, h[0].col), ("es5-syntax", Some(1), Some(31)));
}

#[test]
fn non_utf8_file() {
    let p = proj("enc");
    put(&p, "ui/extra.js", b"var a;\nvar \xff\xfe = 1; let q;");
    let f = run(&p);
    let h = rules(&f, "encoding");
    assert_eq!(h.len(), 1, "{f:?}");
    assert_eq!((h[0].file.as_deref(), h[0].line, h[0].col), (Some("ui/extra.js"), Some(2), Some(5)));
    assert!(rules(&f, "es5-syntax").is_empty(), "file skipped");
}

#[test]
fn css_presto() {
    let p = proj("css");
    put(
        &p,
        "ui/style.css",
        ".app-m-a { display: flex; }\n.app-m-b{DISPLAY:inline-grid;width:calc(1px + 2px);color:var(--x)}\n\
         .app-m-c { -webkit-transition: none; -o-transition: none; clip-path: none; }\n\
         /* display:flex; var(--x) */\n.app-m-d { display: block; margin: 0 }\n.app-m-o-x { color: red }\n",
    );
    let f = run(&p);
    let got: Vec<_> = rules(&f, "css-presto").iter().map(|x| (x.line.unwrap(), x.col.unwrap())).collect();
    assert_eq!(got, vec![(1, 12), (2, 10), (2, 36), (2, 58), (3, 12), (3, 38), (3, 59)], "{f:?}");
}

#[test]
fn css_presto_in_html_style_attr() {
    let p = proj("html");
    put(&p, "ui/page.html", "<div class=\"a\"\n style=\"display:flex\"></div><p style='color:red'></p>\n");
    let f = run(&p);
    let h = rules(&f, "css-presto");
    assert_eq!(h.len(), 1, "{f:?}");
    assert_eq!((h[0].file.as_deref(), h[0].line, h[0].col), (Some("ui/page.html"), Some(2), Some(9)));
}

#[test]
fn ws_literal_port_is_warning() {
    let (_, f) = js("ws", "var a = \"ws://127.0.0.1:2777\"; var b = 'ws://localhost:8750/x'; var c = \"ws://localhost/\";");
    let h = rules(&f, "ws-literal-port");
    assert_eq!(h.len(), 2, "{f:?}");
    assert!(h.iter().all(|x| x.level == Level::Warning));
    assert!(!revised_sdk::lint::has_errors(&f));
}

#[test]
fn hmi_handwritten_is_warning() {
    let p = proj("hand");
    put(&p, "ui/hmi.json", "{}");
    let f = run(&p);
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!((f[0].rule.as_str(), f[0].level), ("hmi-handwritten", Level::Warning));
}

#[test]
fn register_key() {
    let p = proj("reg");
    put(&p, "ui/main.js", "RevisedHMI.register(\"other/x\", {});");
    let f = run(&p);
    assert_eq!(rules(&f, "register-key").len(), 1, "{f:?}");
    assert_eq!(rules(&f, "register-key")[0].level, Level::Error);
    put(&p, "ui/main.js", "RevisedHMI.register( 'm/sub', {});");
    assert!(run(&p).is_empty());
}

#[test]
fn modyaml_findings_pass_through() {
    let p = proj("yaml");
    let y = std::fs::read_to_string(p.join("mod.yaml")).unwrap().replace("74.00.324A", "1.0");
    put(&p, "mod.yaml", y);
    let f = run(&p);
    assert_eq!(f[0].rule, "target-cmu");
}

#[test]
fn build_refuses_on_lint_error_and_ignores_warnings() {
    let p = proj("bld");
    put(&p, "ui/main.js", "RevisedHMI.register(\"m/m\", {});\nlet x = 1;\n");
    let e = revised_sdk::build::build(&p).unwrap_err();
    assert_eq!(e[0].rule, "es5-syntax");
    put(&p, "ui/main.js", "RevisedHMI.register(\"m/m\", {});\nvar u = \"ws://localhost:1\";\n");
    assert!(revised_sdk::build::build(&p).is_ok());
}

#[test]
fn cli_lint_exit_codes() {
    let p = proj("cli");
    let bin = env!("CARGO_BIN_EXE_revised-sdk");
    let run = |args: &[&str]| std::process::Command::new(bin).arg("-C").arg(&p).args(args).output().unwrap();
    assert_eq!(run(&["lint"]).status.code(), Some(0));
    put(&p, "ui/main.js", "RevisedHMI.register(\"m/m\", {});\nlet x;\n");
    let o = run(&["--json", "lint"]);
    assert_eq!(o.status.code(), Some(1));
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!((v["command"].as_str(), v["errors"][0]["rule"].as_str()), (Some("lint"), Some("es5-syntax")));
}
