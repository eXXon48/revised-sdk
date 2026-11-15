//! `setup` (build the SDK image) and `opera install` (sha256-checked download).
use crate::engine::{abs_host_path, host_path, image_tag, Engine};
use crate::report::{env_error_in, Finding, Report};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const OPERA_MANIFEST: &str = include_str!("../../container/opera.sha256");
pub const OPERA_VOLUME: &str = "revised-sdk-opera";

pub struct Entry {
    pub sha: String,
    pub name: String,
    pub url: String,
}

impl Entry {
    pub fn is_pending(&self) -> bool {
        self.sha == "PENDING"
    }
}

/// Lines: `<sha256>  <file name>  <url>`; blank lines and `#` comments skipped.
pub fn parse_manifest(text: &str) -> Result<Vec<Entry>, String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let f: Vec<_> = l.split_whitespace().collect();
            match f[..] {
                [sha, name, url] => Ok(Entry { sha: sha.into(), name: name.into(), url: url.into() }),
                _ => Err(format!("opera.sha256: bad line '{l}'")),
            }
        })
        .collect()
}

/// Artifact for this platform. `image` always wants the Linux x86_64 tarball.
fn pick(entries: Vec<Entry>, os: &str, image: bool) -> Result<Entry, String> {
    let key = if image { "linux" } else { os };
    let pat = match key {
        "linux" => "x86_64.linux.tar.xz",
        "windows" => "Setup_x64.exe",
        "macos" => ".dmg",
        o => return Err(format!("no Opera 12.10 artifact for OS '{o}'")),
    };
    entries
        .into_iter()
        .find(|e| e.name.ends_with(pat))
        .ok_or_else(|| format!("opera.sha256 has no entry for {key}"))
}

/// Host install dir: %LOCALAPPDATA%\revised-sdk\opera, ~/.local/share/..., ~/Library/Application Support/...
pub fn host_dir(os: &str, env: &dyn Fn(&str) -> Option<String>) -> Result<PathBuf, String> {
    let need = |k: &str| env(k).ok_or_else(|| format!("{k} is not set"));
    Ok(match os {
        "windows" => PathBuf::from(need("LOCALAPPDATA")?).join("revised-sdk").join("opera"),
        "macos" => PathBuf::from(need("HOME")?).join("Library/Application Support/revised-sdk/opera"),
        _ => PathBuf::from(need("HOME")?).join(".local/share/revised-sdk/opera"),
    })
}

pub type Fetch<'a> = dyn Fn(&str, &Path) -> Result<(), String> + 'a;

pub fn curl_fetch(url: &str, dest: &Path) -> Result<(), String> {
    let s = Command::new("curl")
        .args(["-fL", "--proto", "=https", "-sS", "-o"])
        .arg(dest)
        .arg(url)
        .status()
        .map_err(|e| format!("cannot run curl: {e}"))?;
    if s.success() {
        Ok(())
    } else {
        Err(format!("download failed: {url}"))
    }
}

/// Download `e` into `dir`, check sha256; on mismatch delete the file and `Err` (refusal).
pub fn download_verified(e: &Entry, dir: &Path, fetch: &Fetch) -> Result<PathBuf, String> {
    let dest = dir.join(&e.name);
    fetch(&e.url, &dest)?;
    let bytes = std::fs::read(&dest).map_err(|x| format!("cannot read {}: {x}", dest.display()))?;
    let got: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
    if got.eq_ignore_ascii_case(&e.sha) {
        Ok(dest)
    } else {
        let _ = std::fs::remove_file(&dest);
        Err(format!("sha256 mismatch for {}: expected {}, got {got}; file deleted", e.name, e.sha))
    }
}

/// Unpack a downloaded tarball into the named volume (never into the image).
pub fn image_install_args(tag: &str, dl_dir: &Path, file: &str) -> Vec<String> {
    vec![
        "run".into(),
        "--rm".into(),
        "-v".into(),
        format!("{OPERA_VOLUME}:/opera"),
        "-v".into(),
        format!("{}:/dl:ro", host_path(dl_dir)),
        tag.into(),
        "tar".into(),
        "xJf".into(),
        format!("/dl/{file}"),
        "-C".into(),
        "/opera".into(),
    ]
}

pub struct OperaOpts<'a> {
    pub manifest: &'a str,
    pub os: &'a str,
    pub image: bool,
    pub host_dir: PathBuf,
    pub engine: Option<&'a Engine>,
    pub json: bool,
    pub fetch: &'a Fetch<'a>,
}

pub fn opera_install(o: &OperaOpts) -> i32 {
    const CMD: &str = "opera install";
    let entry = match parse_manifest(o.manifest).and_then(|v| pick(v, o.os, o.image)) {
        Ok(e) => e,
        Err(m) => return env_error_in(o.json, CMD, &m),
    };
    if entry.is_pending() {
        return env_error_in(
            o.json,
            CMD,
            &format!("opera.sha256 has no hash for {} yet: the controller has to fill hashes", entry.name),
        );
    }
    // image: verify in a temp dir, unpack into the volume. host: keep in the host dir.
    let dir = if o.image {
        std::env::temp_dir().join(format!("revised-sdk-opera-{}", std::process::id()))
    } else {
        o.host_dir.clone()
    };
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return env_error_in(o.json, CMD, &format!("cannot create {}: {e}", dir.display()));
    }
    let file = match download_verified(&entry, &dir, o.fetch) {
        Ok(f) => f,
        Err(m) => return Report::new(CMD, json!({}), vec![Finding::error("opera-sha256", &m)]).emit(o.json),
    };
    let result = if o.image {
        install_image(o, &dir, &entry.name)
    } else if o.os == "linux" {
        run(Command::new("tar").arg("xJf").arg(&file).arg("-C").arg(&o.host_dir))
            .map(|_| format!("unpacked into {}", o.host_dir.display()))
    } else {
        Ok(format!("installer saved: {} (run it yourself; revised-sdk does not run installers)", file.display()))
    };
    if o.image {
        let _ = std::fs::remove_dir_all(&dir);
    }
    match result {
        Ok(msg) => {
            if !o.json {
                println!("{msg}");
            }
            let target = if o.image { "image" } else { "host" };
            Report::new(CMD, json!({"target": target, "detail": msg}), vec![]).emit(o.json)
        }
        Err(m) => env_error_in(o.json, CMD, &m),
    }
}

fn install_image(o: &OperaOpts, dl: &Path, name: &str) -> Result<String, String> {
    let e = o.engine.ok_or("no container engine for --image")?;
    run(Command::new(&e.bin).args(image_install_args(&image_tag(), dl, name)))?;
    Ok(format!("unpacked into volume {OPERA_VOLUME}"))
}

fn run(c: &mut Command) -> Result<(), String> {
    match c.status() {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("{:?} failed ({s})", c.get_program())),
        Err(e) => Err(format!("cannot run {:?}: {e}", c.get_program())),
    }
}

pub struct SetupOpts<'a> {
    pub store: Option<&'a Path>,
    pub sdk: Option<&'a Path>,
    pub engine: Option<&'a Engine>,
    pub dry_run: bool,
    pub json: bool,
}

pub fn build_args(tag: &str, sdk: &Path, store: &Path, rev: &str) -> Vec<String> {
    vec![
        "build".into(),
        "-t".into(),
        tag.into(),
        "--label".into(),
        format!("org.revised.store-rev={rev}"),
        "--build-context".into(),
        format!("store={}", host_path(store)),
        "-f".into(),
        format!("{}/container/Containerfile", host_path(sdk)),
        host_path(sdk),
    ]
}

pub fn setup(o: &SetupOpts) -> i32 {
    const CMD: &str = "setup";
    let Some(store) = o.store else {
        return env_error_in(
            o.json,
            CMD,
            "--store <path to cmu-re-store checkout> is required (phase 1: the store repo is private)",
        );
    };
    // ponytail: default is the source checkout this binary was built from; a released binary needs --sdk
    let sdk = o.sdk.map(Path::to_path_buf).unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join(".."));
    if !sdk.join("container/Containerfile").is_file() {
        let m = format!("no container/Containerfile under {}: pass --sdk <revised-sdk checkout>", sdk.display());
        return env_error_in(o.json, CMD, &m);
    }
    let rev = match Command::new("git").arg("-C").arg(store).args(["rev-parse", "HEAD"]).output() {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => return env_error_in(o.json, CMD, &format!("{} is not a git checkout", store.display())),
    };
    let (sdk_s, store_s) = match abs_host_path(&sdk).and_then(|s| abs_host_path(store).map(|t| (s, t))) {
        Ok(p) => p,
        Err(m) => return env_error_in(o.json, CMD, &m),
    };
    let args = build_args(&image_tag(), Path::new(&sdk_s), Path::new(&store_s), &rev);
    if o.dry_run {
        println!("{}", args.join(" "));
        return 0;
    }
    let Some(e) = o.engine else {
        return env_error_in(o.json, CMD, "no container engine found: install podman or docker");
    };
    match run(Command::new(&e.bin).args(&args)) {
        Ok(()) => Report::new(CMD, json!({"image": image_tag(), "store_rev": rev}), vec![]).emit(o.json),
        Err(m) => env_error_in(o.json, CMD, &m),
    }
}
