//! `manifest.json` — the package control file (Package Format §5).
//!
//! Forward-compat: unknown object fields are ignored; unknown enum values are rejected.
//! Canonical form = fixed struct field order + sorted keys inside the opaque `revert`
//! (`serde_json` without `preserve_order` backs objects with a `BTreeMap`).

use serde::{Deserialize, Serialize};

use crate::RvpError;

/// What a file entry does to the live path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum FileType {
    New,
    Modify,
    /// A symlink at an absolute `live` pointing into the package jail (`target`).
    Link,
}

impl TryFrom<String> for FileType {
    type Error = String;
    fn try_from(s: String) -> Result<Self, String> {
        match s.as_str() {
            "new" => Ok(FileType::New),
            "modify" => Ok(FileType::Modify),
            "link" => Ok(FileType::Link),
            _ => Err(format!("invalid type `{s}` (expected new|modify|link)")),
        }
    }
}

impl From<FileType> for String {
    fn from(t: FileType) -> String {
        match t {
            FileType::New => "new",
            FileType::Modify => "modify",
            FileType::Link => "link",
        }
        .into()
    }
}

/// How the payload is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Apply {
    WholeFile,
    XmlNode,
    ThreeWay,
}

impl TryFrom<String> for Apply {
    type Error = String;
    fn try_from(s: String) -> Result<Self, String> {
        match s.as_str() {
            "whole-file" => Ok(Apply::WholeFile),
            "xml-node" => Ok(Apply::XmlNode),
            "3way" => Ok(Apply::ThreeWay),
            _ => Err(format!(
                "invalid apply `{s}` (expected whole-file|xml-node|3way)"
            )),
        }
    }
}

impl From<Apply> for String {
    fn from(a: Apply) -> String {
        match a {
            Apply::WholeFile => "whole-file",
            Apply::XmlNode => "xml-node",
            Apply::ThreeWay => "3way",
        }
        .into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    /// Relative = inside the package jail `/data_persist/cmu-re/pkg/<name>/`;
    /// absolute = an override outside it (needs `reason`).
    pub live: String,
    /// Core member holding the content; absent only for `type=link`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
    /// `type=link` only: relative jail path the symlink points at.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    pub mode: String,
    pub owner: String,
    pub r#type: FileType,
    pub apply: Apply,
    /// Why an override (absolute `live`) is needed; shown to the curator / on unsigned install.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Opaque to S1 (S4 owns its schema); preserved and re-emitted with sorted keys.
    #[serde(default = "empty_object")]
    pub revert: serde_json::Value,
}

fn empty_object() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub format_version: u32,
    pub name: String,
    pub version: String,
    pub author: String,
    #[serde(rename = "targetCMU")]
    pub target_cmu: String,
    pub category: String,
    pub description: String,
    pub min_daemon: String,
    /// Debian syntax: `a (>= 1.0), b | c`.
    #[serde(default)]
    pub depends: String,
    /// Debian syntax, no `|` alternatives.
    #[serde(default)]
    pub conflicts: String,
    pub files: Vec<FileEntry>,
    /// The HMI must restart for this package to take effect. Emitted only when `true`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub restart: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl Manifest {
    pub fn parse(json: &[u8]) -> Result<Manifest, RvpError> {
        serde_json::from_slice(json).map_err(|e| RvpError::Manifest(e.to_string()))
    }

    /// The exact bytes archived into the core.
    pub fn to_canonical_json(&self) -> Result<Vec<u8>, RvpError> {
        serde_json::to_vec(self).map_err(|e| RvpError::Manifest(e.to_string()))
    }
}
