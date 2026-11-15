//! `.revised/local.toml`: per-machine values (git-ignored). Flat `key = "value"` lines only.
use std::path::{Path, PathBuf};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Local {
    pub baseline: Option<PathBuf>,
    pub unit_host: Option<String>,
    pub unit_key: Option<PathBuf>,
}

fn file(project: &Path) -> PathBuf {
    project.join(".revised/local.toml")
}

fn unquote(v: &str) -> Option<String> {
    let inner = v.trim().strip_prefix('"')?.strip_suffix('"')?;
    Some(inner.replace("\\\"", "\"").replace("\\\\", "\\"))
}

fn quote(v: &str) -> String {
    format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))
}

/// `(key, value)` of a `key = "value"` line, else None (comments, blanks, junk).
fn kv(line: &str) -> Option<(&str, String)> {
    let (k, v) = line.split_once('=')?;
    Some((k.trim(), unquote(v)?))
}

/// Missing or unreadable file = all defaults.
pub fn load(project: &Path) -> Local {
    let text = std::fs::read_to_string(file(project)).unwrap_or_default();
    let mut l = Local::default();
    for (k, v) in text.lines().filter_map(kv) {
        match k {
            "baseline" => l.baseline = Some(v.into()),
            "unit_host" => l.unit_host = Some(v),
            "unit_key" => l.unit_key = Some(v.into()),
            _ => {}
        }
    }
    l
}

/// Rewrite the known keys; every other line (unknown keys, comments) is kept as is.
pub fn save(project: &Path, l: &Local) -> std::io::Result<()> {
    let path = file(project);
    let old = std::fs::read_to_string(&path).unwrap_or_default();
    let known = [
        ("baseline", l.baseline.as_ref().map(|p| p.to_string_lossy().replace('\\', "/"))),
        ("unit_host", l.unit_host.clone()),
        ("unit_key", l.unit_key.as_ref().map(|p| p.to_string_lossy().replace('\\', "/"))),
    ];
    let mut out: Vec<String> = old
        .lines()
        .filter(|ln| !matches!(kv(ln), Some((k, _)) if known.iter().any(|(n, _)| *n == k)))
        .map(String::from)
        .collect();
    out.extend(known.iter().filter_map(|(k, v)| v.as_ref().map(|v| format!("{k} = {}", quote(v)))));
    std::fs::create_dir_all(path.parent().expect("has parent"))?;
    std::fs::write(path, out.join("\n") + "\n")
}
