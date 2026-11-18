//! Package tree <-> deterministic core (Package Format §3, §5).
//! Core = gzip of a canonical ustar; members: `manifest.json` (canonical bytes) + one member
//! per `files[].payload`.

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;

use crate::manifest::{Apply, FileEntry, FileType, Manifest};
use crate::ustar::{ustar_stream, ustar_write, UstarEntry};
use crate::RvpError;

const MANIFEST: &str = "manifest.json";
/// Largest `manifest.json` member a reader buffers.
const MANIFEST_MAX: u64 = 1 << 20;
/// The only firmware a package may target (Package Format §5).
pub const TARGET_CMU: &str = "74.00.324A";
/// The manifest format this reader/packer understands.
pub const FORMAT_VERSION: u32 = 1;

fn err(m: impl Into<String>) -> RvpError {
    RvpError::Package(m.into())
}

/// Path made only of normal components: no empty, `.` or `..` parts.
pub(crate) fn clean_components(p: &str) -> bool {
    p.split('/').all(|c| !c.is_empty() && c != "." && c != "..")
}

/// Payload member: `payload/<clean relative path>`.
fn check_payload_path(p: &str) -> Result<(), RvpError> {
    match p.strip_prefix("payload/") {
        Some(rest) if clean_components(rest) => Ok(()),
        _ => Err(err(format!(
            "unsafe payload path (must be payload/<path>, no '..', '.' or empty parts): {p}"
        ))),
    }
}

/// Trees no package may write, even with a reason: kernel/runtime pseudo-fs and Revised's own
/// state (vanilla/index, store, bin, every jail; a package addresses its own jail relatively).
const FORBIDDEN: &[&str] = &[
    "/proc",
    "/sys",
    "/dev",
    "/tmp",
    "/run",
    "/data_persist/cmu-re",
];

/// `p` is `root` or below it (component-wise).
fn under(p: &str, root: &str) -> bool {
    p.strip_prefix(root)
        .is_some_and(|r| r.is_empty() || r.starts_with('/'))
}

/// Live path: relative (inside the jail) or absolute (override); clean either way.
fn check_live_path(p: &str) -> Result<(), RvpError> {
    if clean_components(p.strip_prefix('/').unwrap_or(p)) {
        Ok(())
    } else {
        Err(err(format!(
            "unsafe live path (no '..', '.' or empty parts): {p}"
        )))
    }
}

/// Package jail rules (Package Format §5 Target-location policy, 2026-11-12).
fn check_jail(f: &FileEntry, jailed_new: &[&str]) -> Result<(), RvpError> {
    let live = f.live.as_str();
    let link = f.r#type == FileType::Link;
    match (&f.payload, link) {
        (None, false) => return Err(err(format!("payload missing: {live}"))),
        (Some(_), true) => return Err(err(format!("type link carries no payload: {live}"))),
        _ => {}
    }
    if f.target.is_some() != link {
        return Err(err(format!(
            "target is only for (and required by) type link: {live}"
        )));
    }
    if !live.starts_with('/') {
        return match f.r#type {
            FileType::New => Ok(()),
            _ => Err(err(format!(
                "only type new may live inside the package jail (relative live): {live}"
            ))),
        };
    }
    if let Some(root) = FORBIDDEN.iter().find(|r| under(live, r)) {
        return Err(err(format!(
            "live path under forbidden tree {root}: {live}"
        )));
    }
    if f.reason.as_deref().is_none_or(|r| r.trim().is_empty()) {
        return Err(err(format!(
            "override outside the package jail needs a reason: {live}"
        )));
    }
    if let Some(t) = &f.target {
        if !(clean_components(t) && jailed_new.iter().any(|n| under(n, t))) {
            return Err(err(format!(
                "link target must be jailed content of this package: {live} -> {t}"
            )));
        }
    }
    Ok(())
}

/// Package name: one filesystem-safe component (it becomes `pkg/<name>/` on the unit).
pub(crate) fn check_name(what: &str, n: &str) -> Result<(), RvpError> {
    let ok = (1..=64).contains(&n.len())
        && n.as_bytes()[0].is_ascii_lowercase() | n.as_bytes()[0].is_ascii_digit()
        && n.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b));
    if ok {
        Ok(())
    } else {
        Err(err(format!(
            "invalid {what} {n:?} (must match [a-z0-9][a-z0-9._-]*, at most 64 bytes)"
        )))
    }
}

/// Manifest-level checks shared by pack and unpack; returns the referenced payload set.
fn validate(m: &Manifest) -> Result<BTreeSet<&str>, RvpError> {
    if m.format_version != FORMAT_VERSION {
        return Err(err(format!(
            "formatVersion {} is not supported (this reader handles {FORMAT_VERSION})",
            m.format_version
        )));
    }
    check_name("package name", &m.name)?;
    crate::deps::parse_rels(&m.depends)?;
    for clause in crate::deps::parse_rels(&m.conflicts)? {
        if clause.len() != 1 {
            return Err(RvpError::Manifest(
                "conflicts must not contain `|` alternatives".into(),
            ));
        }
    }
    if m.target_cmu != TARGET_CMU {
        return Err(err(format!(
            "targetCMU {:?} must be {TARGET_CMU}",
            m.target_cmu
        )));
    }
    if m.files.is_empty() {
        return Err(err("manifest lists no files"));
    }
    let jailed_new: Vec<&str> = m
        .files
        .iter()
        .filter(|f| f.r#type == FileType::New && !f.live.starts_with('/'))
        .map(|f| f.live.as_str())
        .collect();
    let mut payloads = BTreeSet::new();
    let mut lives = BTreeSet::new();
    for f in &m.files {
        check_live_path(&f.live)?;
        check_jail(f, &jailed_new)?;
        if f.r#type != FileType::Modify && f.apply != Apply::WholeFile {
            return Err(err(format!(
                "type new/link requires apply whole-file: {}",
                f.live
            )));
        }
        if let Some(p) = &f.payload {
            check_payload_path(p)?;
            if !payloads.insert(p.as_str()) {
                return Err(err(format!("payload listed twice: {p}")));
            }
        }
        if !lives.insert(f.live.as_str()) {
            return Err(err(format!("live path listed twice: {}", f.live)));
        }
    }
    if let Some(c) = &m.config {
        c.check(m)?;
    }
    Ok(payloads)
}

/// Every regular file under `dir`, as `prefix/<rel>` paths. Symlinks and non-UTF-8 names fail.
fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) -> Result<(), RvpError> {
    let rd = std::fs::read_dir(dir).map_err(|e| err(format!("{}: {e}", dir.display())))?;
    for e in rd {
        let e = e.map_err(|e| err(format!("{}: {e}", dir.display())))?;
        let name = e
            .file_name()
            .into_string()
            .map_err(|n| err(format!("non-UTF-8 name under {prefix}: {n:?}")))?;
        let rel = format!("{prefix}/{name}");
        let ft = e.file_type().map_err(|e| err(format!("{rel}: {e}")))?;
        if ft.is_dir() {
            walk(&e.path(), &rel, out)?;
        } else if ft.is_file() {
            out.push(rel);
        } else {
            return Err(err(format!("not a regular file or directory: {rel}")));
        }
    }
    Ok(())
}

/// Reads `src_dir/manifest.json` + `src_dir/payload/**`, validates, emits the core.
pub fn pack_dir(src_dir: &Path) -> Result<Vec<u8>, RvpError> {
    let raw = std::fs::read(src_dir.join(MANIFEST)).map_err(|e| err(format!("{MANIFEST}: {e}")))?;
    let m = Manifest::parse(&raw)?;
    let wanted = validate(&m)?;

    let mut found = Vec::new();
    let payload_dir = src_dir.join("payload");
    if payload_dir.is_dir() {
        walk(&payload_dir, "payload", &mut found)?;
    }
    let found: BTreeSet<&str> = found.iter().map(String::as_str).collect();
    if let Some(missing) = wanted.difference(&found).next() {
        return Err(err(format!("payload file missing from tree: {missing}")));
    }
    if let Some(orphan) = found.difference(&wanted).next() {
        return Err(err(format!(
            "payload file not listed in manifest files: {orphan}"
        )));
    }

    let mut entries = vec![UstarEntry {
        name: MANIFEST.into(),
        data: m.to_canonical_json()?,
        is_dir: false,
    }];
    for p in wanted {
        let data = std::fs::read(src_dir.join(p)).map_err(|e| err(format!("{p}: {e}")))?;
        entries.push(UstarEntry {
            name: p.into(),
            data,
            is_dir: false,
        });
    }
    // every template renders with the defaults: no unknown or unclosed placeholder ships
    if let Some(c) = &m.config {
        for o in &c.outputs {
            let p = m.files.iter().find(|f| f.live == o.template).and_then(|f| f.payload.as_deref());
            let data = entries.iter().find(|e| Some(e.name.as_str()) == p).map(|e| &e.data);
            let text = data.and_then(|d| std::str::from_utf8(d).ok()).ok_or_else(|| err(format!("template is not UTF-8 text: {}", o.template)))?;
            c.render(text, &c.defaults()).map_err(|e| err(format!("{}: {e}", o.template)))?;
        }
    }
    gzip(&ustar_write(&entries)?)
}

/// Deterministic gzip of a ustar (Package Format §3): mtime 0, no name/comment/extra,
/// OS 255, best compression.
pub fn gzip(tar: &[u8]) -> Result<Vec<u8>, RvpError> {
    let mut e = flate2::GzBuilder::new().write(Vec::new(), flate2::Compression::best());
    e.write_all(tar).map_err(|e| err(format!("gzip: {e}")))?;
    e.finish().map_err(|e| err(format!("gzip: {e}")))
}

/// Strict inverse of [`gzip`]: one gzip member, CRC checked, nothing after it.
pub fn gunzip(core: &[u8]) -> Result<Vec<u8>, RvpError> {
    let mut d = flate2::bufread::GzDecoder::new(core);
    let mut out = vec![];
    d.read_to_end(&mut out).map_err(|e| err(format!("gzip: {e}")))?;
    if !d.into_inner().is_empty() {
        return Err(err("trailing bytes after the gzip core"));
    }
    Ok(out)
}

/// Payload callback of [`unpack_stream`]: `(validated manifest, member name, bounded data)`.
pub type OnPayload<'a> = dyn FnMut(&Manifest, &str, &mut dyn Read) -> Result<(), RvpError> + 'a;

/// Streams a core: gunzip, canonical ustar, validated manifest first, then every payload
/// member to `on` (data it leaves unread is skipped). Returns the manifest and its raw bytes.
fn unpack_inner(core: &mut dyn BufRead, on: &mut OnPayload) -> Result<(Manifest, Vec<u8>), RvpError> {
    let mut gz = flate2::bufread::GzDecoder::new(core);
    let mut head: Option<(Manifest, Vec<u8>, BTreeSet<String>)> = None;
    let mut seen = BTreeSet::new();
    ustar_stream(&mut gz, &mut |name, is_dir, r| {
        let Some((m, _, wanted)) = &head else {
            if name != MANIFEST || is_dir {
                return Err(err("first core member is not manifest.json"));
            }
            let mut raw = vec![];
            r.take(MANIFEST_MAX + 1)
                .read_to_end(&mut raw)
                .map_err(|e| err(format!("{MANIFEST}: {e}")))?;
            if raw.len() as u64 > MANIFEST_MAX {
                return Err(err("manifest.json is larger than 1 MiB"));
            }
            let m = Manifest::parse(&raw)?;
            let wanted = validate(&m)?.into_iter().map(String::from).collect();
            head = Some((m, raw, wanted));
            return Ok(());
        };
        check_payload_path(name)?;
        if is_dir {
            return Err(err(format!("unexpected directory member: {name}")));
        }
        if !wanted.contains(name) {
            return Err(err(format!("core member not listed in manifest files: {name}")));
        }
        seen.insert(name.to_string());
        on(m, name, r)
    })?;
    let (m, raw, wanted) = head.ok_or_else(|| err("empty core"))?;
    if let Some(missing) = wanted.difference(&seen).next() {
        return Err(err(format!("payload file missing from core: {missing}")));
    }
    let rest = gz.into_inner().fill_buf().map_err(|e| err(format!("gzip: {e}")))?;
    if !rest.is_empty() {
        return Err(err("trailing bytes after the gzip core"));
    }
    Ok((m, raw))
}

/// Streams and validates a core (gzip, canonical ustar, manifest + jail rules, member set);
/// `on` sees each payload member's data, so memory does not grow with the package.
pub fn unpack_stream<R: Read>(
    core: R,
    mut on: impl FnMut(&Manifest, &str, &mut dyn Read) -> Result<(), RvpError>,
) -> Result<Manifest, RvpError> {
    Ok(unpack_inner(&mut BufReader::new(core), &mut on)?.0)
}

/// [`unpack_stream`] collected in memory (tools and tests).
pub fn unpack_mem(core: &[u8]) -> Result<(Manifest, Vec<UstarEntry>), RvpError> {
    let mut entries = vec![];
    let m = unpack_stream(core, |_, name, r| {
        let mut data = vec![];
        r.read_to_end(&mut data).map_err(|e| err(format!("{name}: {e}")))?;
        entries.push(UstarEntry { name: name.into(), data, is_dir: false });
        Ok(())
    })?;
    Ok((m, entries))
}

/// Gate for the signing step: [`unpack_stream`] plus a canonical `manifest.json` member, i.e.
/// the core is exactly what `pack_dir` emits for its contents.
pub fn check_signable(core: &[u8]) -> Result<Manifest, RvpError> {
    let (m, raw) = unpack_inner(&mut &core[..], &mut |_, _, _| Ok(()))?;
    if raw != m.to_canonical_json()? {
        return Err(err("manifest.json member is not canonical"));
    }
    Ok(m)
}
