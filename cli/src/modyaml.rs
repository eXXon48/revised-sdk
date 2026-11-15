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
    pub changes: Vec<Value>,
}

/// Parse and validate. Phase 1 refuses `native:` and non-empty `changes:`.
pub fn parse(text: &str) -> Result<ModYaml, Vec<Finding>> {
    let m: ModYaml = serde_yaml::from_str(text).map_err(|e| {
        let f = Finding::error("mod-yaml", &e.to_string());
        vec![match e.location() {
            Some(l) => f.at("mod.yaml", l.line() as u32, l.column() as u32),
            None => f,
        }]
    })?;
    let mut errs = Vec::new();
    if m.target_cmu != TARGET_CMU {
        errs.push(Finding::error(
            "target-cmu",
            &format!("targetCMU {:?} is not supported; the only supported value is {TARGET_CMU:?}", m.target_cmu),
        ));
    }
    if m.native.is_some() {
        errs.push(Finding::error("native-phase2", "`native:` backends are not supported yet (SDK phase 2)"));
    }
    if !m.changes.is_empty() {
        errs.push(Finding::error("changes-phase2", "`changes:` system-mod edits are not supported yet (SDK phase 2)"));
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
