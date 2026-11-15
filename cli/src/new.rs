//! `revised-sdk new <template> <name>`: copy an embedded template into `<base>/<name>/`.
use crate::report::Finding;
use std::path::{Path, PathBuf};

type Files = &'static [(&'static str, &'static [u8])];

/// Embedded templates. Add a file by adding a line; `{{name}}` in text files is replaced.
const APP: Files = &[
    ("mod.yaml", include_bytes!("../../templates/app/mod.yaml")),
    ("README.md", include_bytes!("../../templates/app/README.md")),
    (".gitignore", include_bytes!("../../templates/app/.gitignore")),
    ("ui/main.js", include_bytes!("../../templates/app/ui/main.js")),
    ("ui/style.css", include_bytes!("../../templates/app/ui/style.css")),
    ("ui/icon.png", include_bytes!("../../templates/app/ui/icon.png")),
];

fn template(name: &str) -> Option<Files> {
    match name {
        "app" => Some(APP),
        _ => None,
    }
}

/// Project names are package names and HMI ids: `[a-z0-9][a-z0-9-]{0,31}`.
fn valid_name(n: &str) -> bool {
    (1..=32).contains(&n.len())
        && n.as_bytes()[0] != b'-'
        && n.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn fail(rule: &str, msg: String) -> Vec<Finding> {
    vec![Finding::error(rule, &msg)]
}

pub fn new_project(base: &Path, tpl: &str, name: &str) -> Result<PathBuf, Vec<Finding>> {
    let files = template(tpl).ok_or_else(|| fail("template", format!("unknown template {tpl:?} (available: app)")))?;
    if !valid_name(name) {
        return Err(fail("name", format!("invalid name {name:?}: use [a-z0-9][a-z0-9-]*, at most 32 chars")));
    }
    let dir = base.join(name);
    let nonempty = std::fs::read_dir(&dir).map(|mut d| d.next().is_some()).unwrap_or(false);
    if dir.is_file() || nonempty {
        return Err(fail("exists", format!("{} exists and is not empty", dir.display())));
    }
    for (rel, bytes) in files {
        let out = dir.join(rel);
        let io = |e: std::io::Error| fail("io", format!("{}: {e}", out.display()));
        std::fs::create_dir_all(out.parent().unwrap_or(&dir)).map_err(io)?;
        let data = match std::str::from_utf8(bytes) {
            Ok(t) => t.replace("{{name}}", name).into_bytes(),
            Err(_) => bytes.to_vec(),
        };
        std::fs::write(&out, data).map_err(io)?;
    }
    Ok(dir)
}
