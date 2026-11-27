//! `revised-sdk build`: `mod.yaml` + `ui/**` + `changes:` files -> `dist/<name>-<version>.rvp` (unsigned).
use crate::baseline;
use crate::modyaml::{self, App, ChangeApply, ModYaml, Row};
use crate::report::Finding;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// `ui/hmi.json` (HMI SDK 4.1): one entry per row. `main` is relative to the package `ui/` dir.
pub fn hmi_entries(m: &ModYaml) -> Value {
    let Some(app) = &m.app else { return json!({"apiVersion": 1, "entries": []}) };
    let both = app.rows.len() > 1;
    let main = app.entry.strip_prefix("ui/").unwrap_or(&app.entry);
    if app.rows.is_empty() && app.background {
        // background-only mod: no menu row, sdk.js only loads `main` at HMI start
        let e = json!({"id": m.name, "section": "none", "label": app.title, "main": main, "listen": true});
        return json!({"apiVersion": 1, "entries": [e]});
    }
    let entries: Vec<Value> = app
        .rows
        .iter()
        .map(|r| {
            let (suffix, section) = match r {
                Row::Apps => ("apps", "applications"),
                Row::Entertainment => ("entertainment", "entertainment"),
            };
            let id = if both { format!("{}-{suffix}", m.name) } else { m.name.clone() };
            json!({"id": id, "section": section, "label": app.title, "main": main, "listen": app.background})
        })
        .collect();
    json!({"apiVersion": 1, "entries": entries})
}

fn io(p: &Path, e: impl std::fmt::Display) -> Vec<Finding> {
    vec![Finding::error("io", &format!("{}: {e}", p.display()))]
}

/// Every non-directory under `dir` as `/`-joined relative paths.
fn walk(dir: &Path, rel: &str, out: &mut Vec<String>) -> Result<(), Vec<Finding>> {
    for e in std::fs::read_dir(dir).map_err(|e| io(dir, e))? {
        let e = e.map_err(|e| io(dir, e))?;
        let name = e.file_name().to_string_lossy().into_owned();
        let r = if rel.is_empty() { name } else { format!("{rel}/{name}") };
        if e.file_type().map_err(|x| io(dir, x))?.is_dir() {
            walk(&e.path(), &r, out)?;
        } else {
            out.push(r);
        }
    }
    Ok(())
}

/// Checks the `app:` block, stages `payload/ui/**` (hmi.json always regenerated) and adds
/// one jailed `new` entry per file.
fn app_entries(dir: &Path, m: &ModYaml, app: &App, stage: &Path, entries: &mut Vec<Value>) -> Result<(), Vec<Finding>> {
    let err = |rule: &str, msg: String| vec![Finding::error(rule, &msg)];
    if app.rows.is_empty() && !app.background {
        return Err(err("rows", "`app.rows` is empty: list apps and/or entertainment, or set background: true".into()));
    }
    let main = app.entry.strip_prefix("ui/").filter(|r| !r.is_empty() && !r.contains("..") && !r.starts_with('/'));
    let Some(main) = main else {
        return Err(err("entry", format!("app.entry {:?} must be a path under ui/", app.entry)));
    };
    if !dir.join("ui").join(main).is_file() {
        return Err(err("entry-missing", format!("app.entry {} does not exist", app.entry)));
    }
    let hmi = hmi_entries(m);
    for e in hmi["entries"].as_array().into_iter().flatten() {
        let id = e["id"].as_str().unwrap_or("");
        if id.len() > 32 {
            return Err(err("hmi-id", format!("HMI id {id:?} is longer than 32 chars: shorten `name`")));
        }
    }
    let ui = dir.join("ui");
    let mut files = Vec::new();
    walk(&ui, "", &mut files)?;
    files.retain(|f| f != "hmi.json");
    files.push("hmi.json".into());
    files.sort();
    for f in &files {
        let dst = stage.join("payload/ui").join(f);
        std::fs::create_dir_all(dst.parent().unwrap_or(stage)).map_err(|e| io(&dst, e))?;
        if f == "hmi.json" {
            let text = serde_json::to_vec(&hmi).expect("json serializes");
            std::fs::write(&dst, text).map_err(|e| io(&dst, e))?;
        } else {
            std::fs::copy(ui.join(f), &dst).map_err(|e| io(&dst, e))?;
        }
        entries.push(json!({"live": format!("ui/{f}"), "payload": format!("payload/ui/{f}"),
            "mode": "0644", "owner": "1000:1000", "type": "new", "apply": "whole-file"}));
    }
    Ok(())
}

pub fn build(dir: &Path) -> Result<PathBuf, Vec<Finding>> {
    let found = crate::lint::run(dir);
    if crate::lint::has_errors(&found) {
        return Err(found);
    }
    let m = modyaml::load(dir)?;
    let err = |rule: &str, msg: String| vec![Finding::error(rule, &msg)];
    let stage = dir.join(".revised").join("stage");
    let _ = std::fs::remove_dir_all(&stage);
    let mut entries = Vec::new();
    // `parse` refuses a project with neither an app nor changes.
    if let Some(app) = &m.app {
        app_entries(dir, &m, app, &stage, &mut entries)?;
    }
    for c in &m.changes {
        // lint already refused a live outside the md5 list; never fall back to a default mode
        let Some((mode, owner)) = baseline::mode_of(&c.live) else {
            return Err(err("stock-path", format!("{} has no mode in the stock baseline list", c.live)));
        };
        let payload = format!("payload/stock{}", c.live);
        let dst = stage.join(&payload);
        std::fs::create_dir_all(dst.parent().unwrap_or(&stage)).map_err(|e| io(&dst, e))?;
        std::fs::copy(dir.join(&c.file), &dst).map_err(|e| io(&dst, e))?;
        let apply = match c.apply {
            ChangeApply::WholeFile => "whole-file",
            ChangeApply::ThreeWay => "3way",
        };
        entries.push(json!({"live": c.live, "payload": payload, "mode": mode, "owner": owner,
            "type": "modify", "apply": apply, "reason": c.reason}));
    }
    let description = m.app.as_ref().map_or(&m.name, |a| &a.title);
    // note: author/minDaemon are fixed until mod.yaml grows those keys.
    let mut manifest = json!({"formatVersion": 1, "name": m.name, "version": m.version, "author": "unknown",
        "targetCMU": m.target_cmu, "category": m.category(), "description": description, "minDaemon": "0.1.0",
        "depends": "", "conflicts": "", "files": entries});
    if !m.changes.is_empty() {
        // the HMI reads stock files at start only
        manifest["restart"] = true.into();
    }
    if let Some(c) = &m.config {
        manifest["config"] = c.clone();
    }
    let rv = |e: rvp::RvpError| err("pack", e.to_string());
    let parsed = rvp::manifest::Manifest::parse(&serde_json::to_vec(&manifest).expect("json serializes")).map_err(rv)?;
    let mj = stage.join("manifest.json");
    std::fs::write(&mj, parsed.to_canonical_json().map_err(rv)?).map_err(|e| io(&mj, e))?;

    // same as `rvp-pack <src> <out>` without --sig. The daemon runs the stock-path check
    // (`check_stock`) against the unit's file list at install.
    let core = rvp::pack::pack_dir(&stage).map_err(rv)?;
    rvp::pack::check_signable(&core).map_err(rv)?;
    let bytes = rvp::container::container_write(&core, None).map_err(rv)?;
    let out = dir.join("dist").join(format!("{}-{}.rvp", m.name, m.version));
    std::fs::create_dir_all(out.parent().unwrap_or(dir)).map_err(|e| io(&out, e))?;
    std::fs::write(&out, bytes).map_err(|e| io(&out, e))?;
    let _ = std::fs::remove_dir_all(&stage);
    Ok(out)
}
