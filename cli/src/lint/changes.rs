//! `changes:` checks: each edit must name a regular stock file, and its payload must fit it.
//! With a local baseline tree, three-way diffs are applied to the stock file with the same
//! diff crate and version the daemon uses, so lint and install agree.
use super::{css, line_starts, locate};
use crate::baseline;
use crate::modyaml::{Change, ChangeApply};
use crate::report::Finding;
use std::path::Path;

const PNG_SIG: &[u8] = b"\x89PNG\r\n\x1a\n";

/// Absolute path of a link `target`, resolved against the directory of `link`.
fn resolve(link: &str, target: &str) -> String {
    let base = if target.starts_with('/') { "" } else { link.rsplit_once('/').map_or("", |(d, _)| d) };
    let mut segs: Vec<&str> = Vec::new();
    for s in base.split('/').chain(target.split('/')) {
        match s {
            "" | "." => {}
            ".." => {
                segs.pop();
            }
            s => segs.push(s),
        }
    }
    format!("/{}", segs.join("/"))
}

/// Why `live` cannot be changed, or None for a regular file of the md5 list.
fn stock_path(live: &str, md5: &str) -> Option<String> {
    if live == "/" || live.split('/').any(|s| s == "..") {
        return Some(format!("{live:?} is not a stock file path: name the file, without `..` segments"));
    }
    // A link to a file or to one of the file's parent directories.
    // ponytail: one hop; a target that is itself a link is reported on the next lint run.
    for (target, link) in baseline::LINKS.lines().filter_map(|l| l.split_once(' ')) {
        if let Some(rest) = live.strip_prefix(link).filter(|r| r.is_empty() || r.starts_with('/')) {
            return Some(format!("{live} is a symlink; use {}{rest}", resolve(link, target)));
        }
    }
    (!baseline::in_md5(md5, live)).then(|| format!("{live} is not a regular file of the stock baseline list"))
}

/// `(width, height)` from the IHDR chunk, or None when `b` is not a PNG.
fn png_dims(b: &[u8]) -> Option<(u32, u32)> {
    if !b.starts_with(PNG_SIG) || b.get(12..16)? != b"IHDR" {
        return None;
    }
    let be = |i: usize| Some(u32::from_be_bytes(b.get(i..i + 4)?.try_into().ok()?));
    Some((be(16)?, be(20)?))
}

/// CSS checks on the `+` lines of a unified diff; positions are the diff's own lines and columns.
fn css_added(file: &str, text: &str, out: &mut Vec<Finding>) {
    let (mut added, mut lines, mut in_hunk) = (String::new(), Vec::new(), false);
    for (n, l) in text.split('\n').enumerate() {
        in_hunk |= l.starts_with("@@");
        if let Some(a) = l.strip_prefix('+').filter(|_| in_hunk) {
            added.push_str(a);
            added.push('\n');
            lines.push(n as u32 + 1);
        }
    }
    let c: Vec<char> = added.chars().collect();
    let starts = line_starts(&c);
    for (i, msg) in css::scan(&c) {
        let (l, col) = locate(&starts, i);
        out.push(Finding::error("css-presto", &msg).at(file, lines[l as usize - 1], col + 1));
    }
}

/// The three-way diff `diff` applied to the stock file under `base`, as the daemon applies it.
fn diff_applies(base: &Path, live: &str, diff: &[u8]) -> Result<(), String> {
    let stock = base.join(live.trim_start_matches('/'));
    let at = format!("{}/{}", base.display(), live.trim_start_matches('/'));
    let orig = std::fs::read(&stock).map_err(|e| format!("cannot read the stock file {at}: {e}"))?;
    let p = diffy::Patch::from_bytes(diff).map_err(|e| format!("not a unified diff: {e}"))?;
    if p.hunks().is_empty() {
        return Err("not a unified diff: no hunks".into());
    }
    diffy::apply_bytes(&orig, &p).map(drop).map_err(|e| format!("three-way diff does not apply to {at}: {e}"))
}

/// Findings for `changes` of the project in `dir`; `md5` is the stock list (production: [`baseline::LIST`]).
pub fn check(dir: &Path, changes: &[Change], base: Option<&Path>, md5: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    for c in changes {
        let err = |rule: &str, msg: String| Finding { file: Some(c.file.clone()), ..Finding::error(rule, &msg) };
        if let Some(msg) = stock_path(&c.live, md5) {
            out.push(err("stock-path", msg));
            continue;
        }
        let bytes = match std::fs::read(dir.join(&c.file)) {
            Ok(b) => b,
            Err(e) => {
                out.push(err("change-missing", format!("{}: {e}", c.file)));
                continue;
            }
        };
        match c.apply {
            ChangeApply::WholeFile if c.live.ends_with(".png") => match png_dims(&bytes) {
                None => out.push(err("png-format", format!("{} is not a PNG file, but replaces the PNG {}", c.file, c.live))),
                Some((w, h)) => {
                    if let Some((sw, sh)) = baseline::png_size(&c.live).filter(|s| *s != (w, h)) {
                        out.push(err("png-size", format!("{} is {w}x{h}, stock {} is {sw}x{sh}", c.file, c.live)));
                    }
                }
            },
            ChangeApply::WholeFile if [".css", ".js", ".html"].iter().any(|x| c.live.ends_with(x)) => out.push(Finding {
                file: Some(c.file.clone()),
                ..Finding::warning(
                    "whole-file-text",
                    &format!("{} replaces the whole text file {}: ship only the changed lines with `apply: three-way`", c.file, c.live),
                )
            }),
            ChangeApply::WholeFile => {}
            ChangeApply::ThreeWay => {
                if c.live.ends_with(".css") {
                    css_added(&c.file, &String::from_utf8_lossy(&bytes), &mut out);
                }
                if let Some(b) = base {
                    if let Err(e) = diff_applies(b, &c.live, &bytes) {
                        out.push(err("diff-apply", e));
                    }
                }
            }
        }
    }
    if base.is_none() && changes.iter().any(|c| c.apply == ChangeApply::ThreeWay) {
        out.push(Finding::warning(
            "no-baseline",
            "no local stock baseline: three-way diffs were not applied to the stock files (set one with `revised-sdk baseline check <dir>`)",
        ));
    }
    out
}
