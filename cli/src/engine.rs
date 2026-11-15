//! Container engine detection and re-exec inside the SDK image.
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Podman,
    Docker,
}

#[derive(Clone, Debug)]
pub struct Engine {
    pub kind: Kind,
    pub bin: PathBuf,
}

fn find_in(name: &str, path: &OsStr) -> Option<PathBuf> {
    std::env::split_paths(path)
        .flat_map(|d| [d.join(name), d.join(format!("{name}.exe"))])
        .find(|p| p.is_file())
}

/// Detect against the process PATH.
pub fn detect(override_: Option<&str>) -> Result<Engine, String> {
    detect_in(override_, &std::env::var_os("PATH").unwrap_or_default())
}

/// Detect against an explicit PATH string (podman preferred over docker).
pub fn detect_in(override_: Option<&str>, path: &OsStr) -> Result<Engine, String> {
    let all = [("podman", Kind::Podman), ("docker", Kind::Docker)];
    let want: Vec<_> = match override_ {
        None => all.to_vec(),
        Some(o) => match all.iter().find(|(n, _)| *n == o) {
            Some(x) => vec![*x],
            None => return Err(format!("unknown engine '{o}' (use podman or docker)")),
        },
    };
    want.iter()
        .find_map(|(n, k)| find_in(n, path).map(|bin| Engine { kind: *k, bin }))
        .ok_or_else(|| match override_ {
            Some(o) => format!("engine '{o}' not found on PATH"),
            None => "no container engine found: install podman or docker".into(),
        })
}

/// Windows path -> form podman machine / Docker Desktop accept (`C:\a b` -> `C:/a b`).
pub fn host_path(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

pub fn image_tag() -> String {
    format!("revised-sdk:{}", env!("CARGO_PKG_VERSION"))
}

pub fn image_missing_message(tag: &str) -> String {
    format!("SDK image {tag} not found: run `revised-sdk setup` to install it")
}

pub fn ensure_image(e: &Engine, tag: &str) -> Result<(), String> {
    let ok = Command::new(&e.bin)
        .args(["image", "inspect", tag])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err(image_missing_message(tag))
    }
}

/// Absolute, engine-ready path. `absolute` (not `canonicalize`) avoids `\\?\` on Windows;
/// a relative path would be read by the engine as a named volume.
pub fn abs_host_path(p: &Path) -> Result<String, String> {
    std::path::absolute(p)
        .map(|a| host_path(&a))
        .map_err(|e| format!("cannot resolve path {}: {e}", p.display()))
}

/// Re-run `revised-sdk <args>` inside the image; returns its exit code.
/// `Err` = environment failure (caller reports it, exit 2).
pub fn run_in_image(
    e: &Engine,
    project: &Path,
    baseline: Option<&Path>,
    args: &[String],
) -> Result<i32, String> {
    let tag = image_tag();
    ensure_image(e, &tag)?;
    let mut c = Command::new(&e.bin);
    c.args(["run", "--rm", "-v"])
        .arg(format!("{}:/work", abs_host_path(project)?))
        .args(["-w", "/work", "-v", "revised-sdk-cache:/cache", "-e", "REVISED_SDK_IN_CONTAINER=1"]);
    if let Some(b) = baseline {
        c.arg("-v").arg(format!("{}:/rootfs:ro", abs_host_path(b)?));
    }
    c.arg(&tag).arg("revised-sdk").args(args);
    let s = c.status().map_err(|err| format!("cannot run {}: {err}", e.bin.display()))?;
    Ok(s.code().unwrap_or(2))
}
