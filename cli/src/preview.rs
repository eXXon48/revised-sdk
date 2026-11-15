//! `revised-sdk preview`: a fake unit in one container (`revised-sdk-preview`): the real daemon in
//! test mode, the sim web server, Xvfb and headless Opera 12.10 (from the `revised-sdk-opera`
//! volume). Host side only; the container side is `preview/entry.sh`.
use crate::build;
use crate::engine::{abs_host_path, ensure_image, image_tag, Engine, Kind};
use crate::setup::OPERA_VOLUME;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

pub const CONTAINER: &str = "revised-sdk-preview";
pub const URL: &str = "http://127.0.0.1:8750/jci/gui/index.html";
const EVAL_SECS: u64 = 6;
const UP_SECS: u64 = 90;
/// Prefix of an `Err` that is an environment problem (exit 2), not a failed check (exit 1).
pub const ENV: &str = "(env)";
const NOT_UP: &str = "preview is not running: `revised-sdk preview up`";

pub struct UpOpts {
    /// Start Opera. Off only for tests of the rest (Opera's license needs the developer's yes).
    pub opera: bool,
}

/// Knob/key names -> keycodes the stub page and the HMI SDK understand.
pub fn keycode(name: &str) -> Option<u32> {
    Some(match name {
        "up" => 38,
        "down" => 40,
        "left" => 37,
        "right" => 39,
        "enter" => 13,
        "back" => 8,
        "home" => 72,
        _ => return None,
    })
}

pub fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The ctl FIFO line the injector evals (`js:` prefix = DEV eval bridge).
pub fn eval_line(js: &str) -> String {
    format!("js:eval(atob(\"{}\"))", b64(js.as_bytes()))
}

fn is_reply(line: &str) -> bool {
    line.contains("\"control\":\"eval\"")
}

pub fn count_replies(log: &str) -> usize {
    log.lines().filter(|l| is_reply(l)).count()
}

/// The `n`th (0-based) eval reply in the log: `Ok(value)` / `Err(js error text)`.
pub fn reply(log: &str, n: usize) -> Option<Result<String, String>> {
    let l = log.lines().filter(|l| is_reply(l)).nth(n)?;
    let v: serde_json::Value = serde_json::from_str(&l[l.find('{')?..]).ok()?;
    let val = v["value"].as_str().unwrap_or("").to_string();
    Some(if v["action"] == "ok" {
        Ok(val)
    } else {
        Err(val)
    })
}

pub fn run_args(tag: &str, project: &str, rvp_rel: &str, opera: bool) -> Vec<String> {
    let s = |x: &str| x.to_string();
    vec![
        s("run"),
        s("-d"),
        s("--name"),
        s(CONTAINER),
        s("-p"),
        s("127.0.0.1:2777:2777"),
        s("-p"),
        s("127.0.0.1:8750:8750"),
        s("-v"),
        format!("{project}:/work"),
        s("-v"),
        format!("{OPERA_VOLUME}:/opera:ro"),
        s("-e"),
        format!("RVP=/work/{rvp_rel}"),
        s("-e"),
        format!("START_OPERA={}", opera as u8),
        s(tag),
        s("sh"),
        s("/opt/revised/preview/entry.sh"),
    ]
}

/// `shot` target: a `.png` path inside the project.
pub fn shot_target(rel: &str) -> Result<String, String> {
    let p = Path::new(rel);
    let bad = p.is_absolute()
        || p.components()
            .any(|c| matches!(c, std::path::Component::ParentDir));
    if bad || p.extension().is_none_or(|x| x != "png") || rel.contains('\'') {
        return Err(format!(
            "shot target must be a .png path inside the project, got {rel:?}"
        ));
    }
    Ok(rel.replace('\\', "/"))
}

fn engine_out(e: &Engine, args: &[&str]) -> Result<(bool, String), String> {
    let o = Command::new(&e.bin)
        .args(args)
        .output()
        .map_err(|x| format!("{ENV}cannot run {}: {x}", e.bin.display()))?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    Ok((o.status.success(), text))
}

fn running(e: &Engine) -> bool {
    matches!(engine_out(e, &["inspect", "-f", "{{.State.Running}}", CONTAINER]), Ok((true, t)) if t.trim() == "true")
}

fn logs_text(e: &Engine) -> String {
    engine_out(e, &["logs", CONTAINER])
        .map(|x| x.1)
        .unwrap_or_default()
}

fn tail_lines(text: &str, n: usize) -> String {
    let l: Vec<_> = text.lines().collect();
    l[l.len().saturating_sub(n)..].join("\n")
}

/// Build the project, replace the container, wait until it serves. Returns the page URL.
pub fn up(e: &Engine, project: &Path, o: &UpOpts) -> Result<String, String> {
    let tag = image_tag();
    ensure_image(e, &tag).map_err(|m| format!("{ENV}{m}"))?;
    if o.opera && !engine_out(e, &["volume", "inspect", OPERA_VOLUME])?.0 {
        return Err(format!(
            "{ENV}Opera is not installed: run `revised-sdk opera install --image`"
        ));
    }
    let rvp = build::build(project).map_err(|f| {
        f.iter()
            .map(|x| x.msg.clone())
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    let rel = rvp
        .strip_prefix(project)
        .unwrap_or(&rvp)
        .to_string_lossy()
        .replace('\\', "/");
    // one sim per machine: replace whatever a crash or an earlier `up` left
    let _ = engine_out(e, &["rm", "-f", CONTAINER]);
    let mut args = run_args(
        &tag,
        &abs_host_path(project).map_err(|m| format!("{ENV}{m}"))?,
        &rel,
        o.opera,
    );
    if e.kind == Kind::Podman {
        // `logs` is how eval replies are read: do not depend on journald being reachable
        args.splice(2..2, ["--log-driver".to_string(), "k8s-file".to_string()]);
    }
    let (ok, out) = engine_out(e, &args.iter().map(String::as_str).collect::<Vec<_>>())?;
    if !ok {
        return Err(format!(
            "{ENV}cannot start {CONTAINER} (is another sim on 127.0.0.1:2777 or :8750?): {}",
            out.trim()
        ));
    }
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(UP_SECS) {
        std::thread::sleep(Duration::from_millis(500));
        let l = logs_text(e);
        if !running(e) {
            return Err(format!(
                "preview container stopped:\n{}",
                tail_lines(&l, 15)
            ));
        }
        if l.contains("[preview] installed") && (!o.opera || eval(e, "1").is_ok()) {
            return Ok(URL.into());
        }
    }
    Err(format!(
        "preview not ready after {UP_SECS}s:\n{}",
        tail_lines(&logs_text(e), 15)
    ))
}

pub fn down(e: &Engine) -> Result<(), String> {
    engine_out(e, &["rm", "-f", CONTAINER]).map(|_| ())
}

pub fn url(e: &Engine) -> Result<String, String> {
    if running(e) {
        Ok(URL.into())
    } else {
        Err(NOT_UP.into())
    }
}

fn ctl(e: &Engine, line: &str) -> Result<(), String> {
    if !running(e) {
        return Err(NOT_UP.into());
    }
    let sh = "printf '%s\\n' \"$1\" > /tmp/rvp-ctl";
    let (ok, out) = engine_out(e, &["exec", CONTAINER, "sh", "-c", sh, "sh", line])?;
    if ok {
        Ok(())
    } else {
        Err(out)
    }
}

pub fn key(e: &Engine, name: &str) -> Result<u32, String> {
    let c = keycode(name)
        .ok_or_else(|| format!("unknown key {name:?} (up down left right enter back home)"))?;
    ctl(e, &c.to_string())?;
    Ok(c)
}

fn eval_within(e: &Engine, js: &str, secs: u64) -> Result<String, String> {
    let before = count_replies(&logs_text(e));
    ctl(e, &eval_line(js))?;
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(secs) {
        if let Some(r) = reply(&logs_text(e), before) {
            return r.map_err(|m| format!("script error: {m}"));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Err(format!(
        "no reply from the page within {secs}s (is Opera running?)"
    ))
}

/// Eval `js` in the HMI page through the DEV eval bridge.
pub fn eval(e: &Engine, js: &str) -> Result<String, String> {
    eval_within(e, js, EVAL_SECS)
}

/// Screenshot of the Xvfb screen, written to `<project>/<rel>` through the /work mount.
pub fn shot(e: &Engine, project: &Path, rel: &str) -> Result<PathBuf, String> {
    let rel = shot_target(rel)?;
    if !running(e) {
        return Err(NOT_UP.into());
    }
    let dest = format!("/work/{rel}");
    let dir = dest.rsplit_once('/').map_or("/work", |d| d.0);
    let cmd = format!("mkdir -p '{dir}' && import -display :99 -window root '{dest}'");
    let (ok, out) = engine_out(e, &["exec", CONTAINER, "sh", "-c", &cmd])?;
    if ok {
        Ok(project.join(rel))
    } else {
        Err(out.trim().to_string())
    }
}

/// Last `n` lines of daemon/sim output, then (when the page answers) the HMI log panel.
pub fn logs(e: &Engine, n: usize) -> Result<Vec<String>, String> {
    if !running(e) {
        return Err(NOT_UP.into());
    }
    let mut v: Vec<String> = tail_lines(&logs_text(e), n)
        .lines()
        .map(String::from)
        .collect();
    if let Ok(h) = eval_within(e, "document.getElementById('log').textContent", 2) {
        v.push("--- HMI log ---".into());
        v.extend(tail_lines(&h, n).lines().map(String::from));
    }
    Ok(v)
}
