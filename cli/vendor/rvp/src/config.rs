//! Package config (manifest `config`, optional): fields the store shows as settings, the values
//! the user picks, and `{{ key }}` templates rendered into the package's own jail files.
//! No field takes free text, so a value can never break the output syntax: no escaping, and the
//! output format is never known here.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::manifest::{FileType, Manifest};
use crate::RvpError;

/// Field key -> value (select: option value string, range/number: integer, switch: bool).
pub type Values = BTreeMap<String, Value>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub fields: Vec<Field>,
    pub outputs: Vec<Output>,
    #[serde(default)]
    pub apply: ApplyMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub key: String,
    pub label: String,
    #[serde(flatten)]
    pub kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Kind {
    Select { options: Vec<Opt>, default: String },
    Range { min: i64, max: i64, step: i64, default: i64 },
    Number { min: i64, max: i64, default: i64 },
    Switch {
        default: bool,
        #[serde(default = "on")]
        on: String,
        #[serde(default = "off")]
        off: String,
    },
}

fn on() -> String {
    "true".into()
}
fn off() -> String {
    "false".into()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Opt {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Output {
    pub template: String,
    pub path: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApplyMode {
    #[default]
    None,
    RestartService,
    Reboot,
}

fn err(m: impl Into<String>) -> RvpError {
    RvpError::Manifest(format!("config: {}", m.into()))
}

impl Field {
    fn default_value(&self) -> Value {
        match &self.kind {
            Kind::Select { default, .. } => default.clone().into(),
            Kind::Range { default, .. } | Kind::Number { default, .. } => (*default).into(),
            Kind::Switch { default, .. } => (*default).into(),
        }
    }

    /// The string written into templates for `v`, or `None` if `v` is not valid for this field.
    fn out(&self, v: &Value) -> Option<String> {
        match &self.kind {
            Kind::Select { options, .. } => {
                let s = v.as_str()?;
                options.iter().any(|o| o.value == s).then(|| s.to_string())
            }
            Kind::Range { min, max, step, .. } => {
                let n = v.as_i64()?;
                (n >= *min && n <= *max && (n - min) % step == 0).then(|| n.to_string())
            }
            Kind::Number { min, max, .. } => {
                let n = v.as_i64()?;
                (n >= *min && n <= *max).then(|| n.to_string())
            }
            Kind::Switch { on, off, .. } => Some(if v.as_bool()? { on.clone() } else { off.clone() }),
        }
    }

    fn check(&self) -> Result<(), RvpError> {
        let k = &self.key;
        let mut c = k.chars();
        if !(c.next().is_some_and(|c| c.is_ascii_lowercase())
            && c.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'))
        {
            return Err(err(format!("bad key {k:?} (want [a-z][a-z0-9_]*)")));
        }
        if self.label.trim().is_empty() {
            return Err(err(format!("{k}: blank label")));
        }
        match &self.kind {
            Kind::Select { options, .. } => {
                if options.len() < 2 {
                    return Err(err(format!("{k}: select needs at least 2 options")));
                }
                let mut seen = BTreeSet::new();
                for o in options {
                    if o.label.trim().is_empty() || !seen.insert(&o.value) {
                        return Err(err(format!("{k}: blank option label or duplicate value {:?}", o.value)));
                    }
                }
            }
            Kind::Range { min, max, step, .. } => {
                if !(min < max && *step > 0 && (max - min) % step == 0) {
                    return Err(err(format!("{k}: range needs min < max, step > 0, (max - min) a multiple of step")));
                }
            }
            Kind::Number { min, max, .. } => {
                if min > max {
                    return Err(err(format!("{k}: number needs min <= max")));
                }
            }
            Kind::Switch { .. } => {}
        }
        if self.out(&self.default_value()).is_none() {
            return Err(err(format!("{k}: default is not a valid value")));
        }
        Ok(())
    }
}

impl Config {
    /// Schema rules; `m` supplies the package's files (templates must be its jailed `type=new`
    /// lives, outputs must not be any of its lives).
    pub fn check(&self, m: &Manifest) -> Result<(), RvpError> {
        if self.fields.is_empty() {
            return Err(err("no fields"));
        }
        let mut keys = BTreeSet::new();
        for f in &self.fields {
            f.check()?;
            if !keys.insert(&f.key) {
                return Err(err(format!("key listed twice: {}", f.key)));
            }
        }
        let mut paths = BTreeSet::new();
        for o in &self.outputs {
            let tpl_ok = m.files.iter().any(|f| f.live == o.template && f.r#type == FileType::New && !f.live.starts_with('/'));
            if !tpl_ok {
                return Err(err(format!("template is not a jailed type=new file of the package: {}", o.template)));
            }
            if !crate::pack::clean_components(&o.path) {
                return Err(err(format!("unsafe output path (jail-relative, no '..', '.' or empty parts): {}", o.path)));
            }
            if m.files.iter().any(|f| f.live == o.path) {
                return Err(err(format!("output path is a package file: {}", o.path)));
            }
            if !paths.insert(&o.path) {
                return Err(err(format!("output path listed twice: {}", o.path)));
            }
        }
        // jail paths taken by payload files and the other outputs: an output may not be one of
        // them, a directory of one, under one, or have its render temp `<path>.cfgtmp` be one
        let lives = m.files.iter().filter(|f| !f.live.starts_with('/')).map(|f| f.live.as_str());
        for (i, o) in self.outputs.iter().enumerate() {
            let others = self.outputs.iter().enumerate().filter(|(k, _)| *k != i).map(|(_, x)| x.path.as_str());
            let p = o.path.as_str();
            let tmp = format!("{p}.cfgtmp");
            if let Some(n) = lives.clone().chain(others).find(|n| {
                *n == p || *n == tmp || n.strip_prefix(p).is_some_and(|r| r.starts_with('/')) || p.strip_prefix(*n).is_some_and(|r| r.starts_with('/'))
            }) {
                return Err(err(format!("output path {p} collides with {n}")));
            }
        }
        Ok(())
    }

    pub fn defaults(&self) -> Values {
        self.fields.iter().map(|f| (f.key.clone(), f.default_value())).collect()
    }

    /// Exactly the declared keys, each valid for its field.
    pub fn check_values(&self, v: &Values) -> Result<(), RvpError> {
        if let Some(k) = v.keys().find(|k| !self.fields.iter().any(|f| &f.key == *k)) {
            return Err(err(format!("unknown key {k}")));
        }
        for f in &self.fields {
            match v.get(&f.key) {
                None => return Err(err(format!("missing value for {}", f.key))),
                Some(x) if f.out(x).is_none() => return Err(err(format!("invalid value for {}: {x}", f.key))),
                _ => {}
            }
        }
        Ok(())
    }

    /// Old values that still validate, defaults for the rest; undeclared keys dropped.
    pub fn merge(&self, old: &Values) -> Values {
        self.fields
            .iter()
            .map(|f| {
                let v = old.get(&f.key).filter(|x| f.out(x).is_some()).cloned().unwrap_or_else(|| f.default_value());
                (f.key.clone(), v)
            })
            .collect()
    }

    /// `tpl` with every `{{ key }}` replaced by the field's output string for `v[key]`.
    pub fn render(&self, tpl: &str, v: &Values) -> Result<String, RvpError> {
        let mut out = String::with_capacity(tpl.len());
        let mut rest = tpl;
        while let Some(i) = rest.find("{{") {
            out.push_str(&rest[..i]);
            let after = &rest[i + 2..];
            let j = after.find("}}").ok_or_else(|| err("`{{` without `}}`"))?;
            let key = after[..j].trim();
            let f = self.fields.iter().find(|f| f.key == key).ok_or_else(|| err(format!("unknown placeholder {{{{ {key} }}}}")))?;
            let s = v.get(key).and_then(|x| f.out(x)).ok_or_else(|| err(format!("no valid value for {key}")))?;
            out.push_str(&s);
            rest = &after[j + 2..];
        }
        out.push_str(rest);
        Ok(out)
    }
}
