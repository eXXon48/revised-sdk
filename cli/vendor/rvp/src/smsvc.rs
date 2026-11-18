//! SM (system manager) service declarations of a package: which `<service>` nodes its
//! `/jci/sm/sm.conf` + `/jci/sm/sm_WCP.conf` xml-node payloads insert.

use std::collections::{BTreeMap, BTreeSet};

use crate::manifest::{Apply, FileType, Manifest};
use crate::ustar::UstarEntry;
use crate::RvpError;

pub const SM_FILES: [&str; 2] = ["/jci/sm/sm.conf", "/jci/sm/sm_WCP.conf"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmService {
    pub name: String,
    /// `autorun="no"`: the daemon starts/stops it on demand instead of SM at boot.
    pub on_demand: bool,
}

fn err(m: String) -> RvpError {
    RvpError::Package(m)
}

/// name -> on_demand for one payload; insert ops only, one `<service>` per node.
fn parse_file(live: &str, data: &[u8]) -> Result<BTreeMap<String, bool>, RvpError> {
    let v: serde_json::Value =
        serde_json::from_slice(data).map_err(|e| err(format!("{live}: payload not JSON: {e}")))?;
    let ops = v["ops"].as_array().ok_or_else(|| err(format!("{live}: payload has no ops")))?;
    let mut out = BTreeMap::new();
    for op in ops {
        let node = op["insert"]["node"]
            .as_str()
            .ok_or_else(|| err(format!("{live}: only insert ops are allowed in an SM file")))?;
        let doc = roxmltree::Document::parse(node)
            .map_err(|e| err(format!("{live}: node is not one well-formed element: {e}")))?;
        let r = doc.root_element();
        if r.tag_name().name() != "service" {
            return Err(err(format!("{live}: inserted node is not a <service>")));
        }
        if r.descendants().any(|n| n != r && n.has_tag_name("service")) {
            return Err(err(format!("{live}: more than one <service> in one node")));
        }
        let name = r.attribute("name").ok_or_else(|| err(format!("{live}: service has no name")))?;
        // the name reaches `smctl -n` as an argument: no leading '-', no shell/xml surprises
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return Err(err(format!("{live}: service name {name:?} must match [A-Za-z0-9_]+")));
        }
        let on_demand = match r.attribute("autorun") {
            Some("yes") => false,
            Some("no") => true,
            other => {
                return Err(err(format!("{live}: service {name}: autorun must be yes|no, got {other:?}")))
            }
        };
        if out.insert(name.to_string(), on_demand).is_some() {
            return Err(err(format!("{live}: service {name} inserted twice")));
        }
    }
    Ok(out)
}

/// SM services the package declares, sorted by name; empty when it touches neither SM file.
/// Both files must declare the same services with the same `autorun`.
pub fn sm_services(m: &Manifest, entries: &[UstarEntry]) -> Result<Vec<SmService>, RvpError> {
    let mut per_file: Vec<BTreeMap<String, bool>> = Vec::new();
    for live in SM_FILES {
        let Some(f) = m.files.iter().find(|f| {
            f.live == live && f.r#type == FileType::Modify && f.apply == Apply::XmlNode
        }) else {
            continue;
        };
        let p = f.payload.as_deref().ok_or_else(|| err(format!("{live}: payload missing")))?;
        let e = entries
            .iter()
            .find(|e| e.name == p)
            .ok_or_else(|| err(format!("{live}: payload entry missing: {p}")))?;
        per_file.push(parse_file(live, &e.data)?);
    }
    match per_file.as_slice() {
        [] => Ok(vec![]),
        [_] => Err(err("SM services must be declared in both sm.conf and sm_WCP.conf".into())),
        [a, b] if a != b => Err(err(
            "sm.conf and sm_WCP.conf declare different services or autorun".into(),
        )),
        [a, _] => Ok(a.iter().map(|(name, &on_demand)| SmService { name: name.clone(), on_demand }).collect()),
        _ => unreachable!(),
    }
}

/// Every `<service name=...>` in an SM config file (`sm.conf` / `sm_WCP.conf` text).
pub fn service_names(doc: &str) -> Result<BTreeSet<String>, RvpError> {
    let d = roxmltree::Document::parse(doc).map_err(|e| err(format!("SM config not well-formed: {e}")))?;
    Ok(d.descendants()
        .filter(|n| n.has_tag_name("service"))
        .filter_map(|n| n.attribute("name").map(String::from))
        .collect())
}
