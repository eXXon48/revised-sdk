//! `mod.yaml`: the project description `build` compiles into a `.rvp`.
use crate::report::Finding;
use serde::Deserialize;
use serde_yaml::Value;
use std::path::Path;

/// The only firmware a package may target.
pub const TARGET_CMU: &str = "74.00.324A";

#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Row {
    Apps,
    Entertainment,
}

#[derive(Deserialize, Clone, Debug)]
pub struct App {
    pub title: String,
    pub entry: String,
    pub rows: Vec<Row>,
    #[serde(default)]
    pub background: bool,
}

/// How a change is applied to the stock file on the unit.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeApply {
    WholeFile,
    ThreeWay,
}

/// One edit to a stock file: `file` (project path) replaces or patches `live` (absolute path on the unit).
#[derive(Deserialize, Clone, Debug)]
pub struct Change {
    pub live: String,
    pub file: String,
    pub apply: ChangeApply,
    pub reason: String,
}

/// Catalog categories a package may declare.
pub const CATEGORIES: [&str; 3] = ["Apps", "Skins", "System mods"];

#[derive(Deserialize, Clone, Debug)]
pub struct ModYaml {
    pub name: String,
    pub version: String,
    #[serde(rename = "targetCMU")]
    pub target_cmu: String,
    pub app: Option<App>,
    pub native: Option<Value>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub changes: Vec<Change>,
    /// Catalog category; see [`ModYaml::category`] for the default.
    pub category: Option<String>,
    /// Package settings, copied as is into the manifest's `config` (the package checks validate it).
    pub config: Option<serde_json::Value>,
}

impl ModYaml {
    /// The declared category, or `Apps` for a project with an app and `System mods` without.
    pub fn category(&self) -> &str {
        match &self.category {
            Some(c) => c,
            None if self.app.is_some() => "Apps",
            None => "System mods",
        }
    }
}

/// Project names are package names and HMI ids: `[a-z0-9][a-z0-9-]{0,31}`.
pub fn valid_name(n: &str) -> bool {
    (1..=32).contains(&n.len())
        && n.as_bytes()[0] != b'-'
        && n.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Versions end up in file names, remote paths and JSON: a strict charset keeps all three safe.
pub fn valid_version(v: &str) -> bool {
    (1..=64).contains(&v.len())
        && !v.starts_with(['.', '-'])
        && !v.contains("..")
        && v.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'+' | b'-'))
}

/// Parse and validate the shape. Phase 1 refuses `native:`.
pub fn parse(text: &str) -> Result<ModYaml, Vec<Finding>> {
    let m: ModYaml = serde_yaml::from_str(text).map_err(|e| {
        let f = Finding::error("mod-yaml", &e.to_string());
        vec![match e.location() {
            Some(l) => f.at("mod.yaml", l.line() as u32, l.column() as u32),
            None => f,
        }]
    })?;
    let mut errs = Vec::new();
    if !valid_name(&m.name) {
        errs.push(Finding::error("name", &format!("invalid name {:?}: use [a-z0-9][a-z0-9-]*, at most 32 chars", m.name)));
    }
    if !valid_version(&m.version) {
        errs.push(Finding::error(
            "version",
            &format!("invalid version {:?}: use [0-9A-Za-z.+-], at most 64 chars, not starting with `.` or `-`, no `..`", m.version),
        ));
    }
    if m.target_cmu != TARGET_CMU {
        errs.push(Finding::error(
            "target-cmu",
            &format!("targetCMU {:?} is not supported; the only supported value is {TARGET_CMU:?}", m.target_cmu),
        ));
    }
    if m.native.is_some() {
        errs.push(Finding::error("native-phase2", "`native:` backends are not supported yet (SDK phase 2)"));
    }
    if m.app.is_none() && m.changes.is_empty() {
        errs.push(Finding::error("no-app", "mod.yaml needs an `app:` block, a `changes:` list, or both"));
    }
    if let Some(c) = &m.category {
        if !CATEGORIES.contains(&c.as_str()) {
            errs.push(Finding::error("category", &format!("category {c:?} is not supported; use one of {CATEGORIES:?}")));
        }
    }
    for (i, c) in m.changes.iter().enumerate() {
        if !c.live.starts_with('/') {
            errs.push(Finding::error("change-live", &format!("changes[{i}].live {:?} must be an absolute path on the unit", c.live)));
        }
        let f = &c.file;
        if f.is_empty() || f.starts_with('/') || f.contains("..") || f == "ui" || f.starts_with("ui/") {
            errs.push(Finding::error(
                "change-file",
                &format!("changes[{i}].file {f:?} must be a relative project path outside ui/, without `..`"),
            ));
        }
        if c.reason.trim().is_empty() {
            errs.push(Finding::error("change-reason", &format!("changes[{i}].reason is empty: say why the stock file is changed")));
        }
    }
    if errs.is_empty() {
        Ok(m)
    } else {
        Err(errs)
    }
}

pub fn load(dir: &Path) -> Result<ModYaml, Vec<Finding>> {
    let text = std::fs::read_to_string(dir.join("mod.yaml"))
        .map_err(|e| vec![Finding::error("mod-yaml", &format!("cannot read mod.yaml in {}: {e}", dir.display()))])?;
    parse(&text)
}
