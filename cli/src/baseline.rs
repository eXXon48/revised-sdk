//! `baseline check`: md5 of every file in the published list against a stock rootfs dir.
//! The embedded list holds Mazda path names and hashes only, never file content.
use crate::local;
use crate::report::{Finding, Report};
use md5::{Digest, Md5};
use serde_json::json;
use std::io::Read;
use std::path::Path;

pub const LIST: &str = include_str!("../../baseline/74.00.324A.md5");
pub const PNG_SIZES: &str = include_str!("../../baseline/74.00.324A.png-sizes");
pub const MODES: &str = include_str!("../../baseline/74.00.324A.modes");
/// `<target> <link>` per line; a relative target is relative to the link's directory.
pub const LINKS: &str = include_str!("../../baseline/74.00.324A.links");

/// Is `path` a regular file of the stock baseline (an entry of the md5 list)?
pub fn in_md5_list(path: &str) -> bool {
    in_md5(LIST, path)
}

/// Is `path` an entry of the md5 list `list` (`<md5>  <path>` lines)?
pub fn in_md5(list: &str, path: &str) -> bool {
    list.lines().filter_map(|l| l.split_once("  ")).any(|(_, p)| p == path)
}

/// Pixel size `(width, height)` of a stock PNG, from the published list.
pub fn png_size(path: &str) -> Option<(u32, u32)> {
    PNG_SIZES.lines().find_map(|l| {
        let mut it = l.split(' ');
        (it.next()? == path).then_some(())?;
        Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?))
    })
}

/// `(mode, "uid:gid")` of a stock file, e.g. `("0644", "1000:1000")`.
pub fn mode_of(path: &str) -> Option<(String, String)> {
    MODES.lines().find_map(|l| {
        let mut it = l.splitn(3, ' ');
        let (m, o) = (it.next()?, it.next()?);
        (it.next()? == path).then(|| (m.to_string(), o.to_string()))
    })
}

fn md5_file(p: &Path) -> std::io::Result<String> {
    let mut f = std::fs::File::open(p)?;
    let (mut h, mut buf) = (Md5::new(), [0u8; 65536]);
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Warn when `dir` looks like a Windows drive (the rootfs needs symlinks and modes).
pub fn ntfs_hint(dir: &Path) -> Option<Finding> {
    let s = dir.to_string_lossy();
    let b = s.as_bytes();
    let win = b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/');
    let r = s.strip_prefix("/mnt").unwrap_or(&s).as_bytes();
    let posix_mount = r.len() >= 2 && r[0] == b'/' && r[1].is_ascii_alphabetic() && (r.len() == 2 || r[2] == b'/');
    (win || posix_mount).then(|| {
        Finding::warning(
            "ntfs",
            "baseline dir is on a Windows drive; extract the rootfs onto a POSIX file system (symlinks and modes are needed)",
        )
    })
}

/// Hash every `<md5>  /<path>` entry of `list` under `dir`. A symlink is followed only
/// if it stays inside `dir`; otherwise the file counts as missing.
pub fn check(dir: &Path, list: &str) -> Report {
    let root = std::fs::canonicalize(dir).ok();
    let (mut n, mut missing, mut mismatch) = (0, vec![], vec![]);
    for (want, path) in list.lines().filter_map(|l| l.split_once("  ")) {
        n += 1;
        let p = dir.join(path.trim_start_matches('/'));
        let inside = match (&root, std::fs::canonicalize(&p)) {
            (Some(r), Ok(c)) => c.starts_with(r),
            _ => false,
        };
        match if inside { md5_file(&p).ok() } else { None } {
            None => missing.push(path.to_string()),
            Some(got) if got != want => mismatch.push(path.to_string()),
            _ => {}
        }
    }
    let errs = missing
        .iter()
        .map(|p| ("baseline-missing", p, "file missing from baseline dir"))
        .chain(mismatch.iter().map(|p| ("baseline-mismatch", p, "md5 differs from the published baseline")))
        .map(|(rule, p, m)| Finding { file: Some(p.clone()), ..Finding::error(rule, m) })
        .collect();
    Report::new("baseline check", json!({"checked": n, "missing": missing, "mismatch": mismatch}), errs)
}

/// `check` + ntfs hint; on ok records the absolute dir in the project's `.revised/local.toml`.
pub fn check_cmd(project: &Path, dir: &Path, list: &str) -> Report {
    let mut r = check(dir, list);
    r.errors.extend(ntfs_hint(dir));
    if r.ok {
        let abs = std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
        let mut l = local::load(project);
        l.baseline = Some(abs);
        if let Err(e) = local::save(project, &l) {
            r.errors.push(Finding::warning("local-toml", &format!("cannot record baseline path: {e}")));
        }
    }
    r
}
