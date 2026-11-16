//! `revised-sdk push` and `unit logs|shot|eval`: the unit over dev SSH (system `ssh`/`scp`, key-only,
//! port 36000). `push` copies the built `.rvp` into the unit's push dir, then asks the daemon
//! (WS 2777, reached through an SSH tunnel) for `install-push`.
use crate::preview::{count_replies, eval_line, reply};
use crate::report::{self, Finding};
use crate::{build, local, modyaml};
use serde_json::{json, Value};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tungstenite::{stream::MaybeTlsStream, Message};

pub const PORT: &str = "36000";
const PUSH_DIR: &str = "/tmp/revised-push";
const UNIT_LOG: &str = "/var/log/running_log/log";
const DAEMON_LOG: &str = "/tmp/revised-daemon.log";
const REMOTE_SHOT: &str = "/tmp/revised-shot.png";
const EVAL_SECS: u64 = 6;
/// Prefix of an error string that is an environment problem (exit 2), not a failed check (exit 1).
const ENV: &str = "(env)";
/// Test seam: connect the daemon WS here instead of through the tunnel.
pub const WS_URL_ENV: &str = "REVISED_SDK_WS_URL";

pub struct Unit {
    pub host: String,
    pub key: PathBuf,
}

/// Options shared by every ssh/scp call. `ConnectTimeout` keeps an unreachable unit from hanging.
fn common(key: &Path, port_flag: &str) -> Vec<String> {
    let s = |x: &str| x.to_string();
    vec![
        s(port_flag),
        s(PORT),
        s("-i"),
        key.to_string_lossy().into_owned(),
        s("-o"),
        s("BatchMode=yes"),
        s("-o"),
        s("ConnectTimeout=10"),
        s("-o"),
        s("StrictHostKeyChecking=accept-new"),
        s("-o"),
        s("ServerAliveInterval=5"),
        s("-o"),
        s("ServerAliveCountMax=1"),
    ]
}

/// `ssh -p 36000 -i <key> -o BatchMode=yes ... <host> <cmd>`
pub fn ssh_args(host: &str, key: &Path, cmd: &str) -> Vec<String> {
    let mut a = common(key, "-p");
    a.extend([host.to_string(), cmd.to_string()]);
    a
}

/// `ssh ... -L <port>:127.0.0.1:2777 -N <host>`
pub fn tunnel_args(host: &str, key: &Path, port: u16) -> Vec<String> {
    let mut a = common(key, "-p");
    a.extend([
        "-o".into(),
        "ExitOnForwardFailure=yes".into(),
        "-L".into(),
        format!("{port}:127.0.0.1:2777"),
        "-N".into(),
        host.to_string(),
    ]);
    a
}

/// Local deadline for a normal ssh/scp call; `scp` of a package may take longer.
const SSH_SECS: u64 = 60;
const SCP_SECS: u64 = 300;
pub(crate) const TIMED_OUT: &str = "timed out after";

/// Run `prog`; kill it when `secs` pass (a stuck remote command must not hang the CLI).
pub(crate) fn run(prog: &str, args: &[String], secs: u64) -> Result<String, String> {
    let mut c = Command::new(prog)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            format!("{ENV}cannot run `{prog}` (is OpenSSH installed and on PATH?): {e}")
        })?;
    fn drain(p: Option<impl std::io::Read + Send + 'static>) -> std::thread::JoinHandle<String> {
        std::thread::spawn(move || {
            let mut b = Vec::new();
            if let Some(mut p) = p {
                let _ = p.read_to_end(&mut b);
            }
            String::from_utf8_lossy(&b).into_owned()
        })
    }
    let (out, err) = (drain(c.stdout.take()), drain(c.stderr.take()));
    let t0 = Instant::now();
    let status = loop {
        if let Some(st) = c.try_wait().map_err(|e| format!("{ENV}{prog}: {e}"))? {
            break Some(st);
        }
        if t0.elapsed() >= Duration::from_secs(secs) {
            let _ = c.kill();
            let _ = c.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let (out, err) = (
        out.join().unwrap_or_default(),
        err.join().unwrap_or_default(),
    );
    let Some(st) = status else {
        return Err(format!("{ENV}`{prog}` {TIMED_OUT} {secs}s: {}", err.trim()));
    };
    if st.success() {
        return Ok(out);
    }
    let code = st.code().map_or("signal".into(), |c| c.to_string());
    Err(format!("{ENV}`{prog}` failed ({code}): {}", err.trim()))
}

impl Unit {
    pub fn ssh(&self, cmd: &str) -> Result<String, String> {
        self.ssh_for(cmd, SSH_SECS)
    }
    fn ssh_for(&self, cmd: &str, secs: u64) -> Result<String, String> {
        run("ssh", &ssh_args(&self.host, &self.key, cmd), secs)
    }
    fn scp(&self, from: &str, to: &str) -> Result<(), String> {
        let mut a = common(&self.key, "-P");
        a.extend([from.to_string(), to.to_string()]);
        run("scp", &a, SCP_SECS).map(|_| ())
    }
    fn remote(&self, path: &str) -> String {
        format!("{}:{path}", self.host)
    }
}

/// base64 decode, skipping everything outside the alphabet (PEM line breaks, padding).
fn unb64(s: &str) -> Vec<u8> {
    let (mut out, mut acc, mut bits) = (Vec::new(), 0u32, 0);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => continue,
        };
        acc = acc << 6 | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out
}

/// The unit's OpenSSH 5.9 has no ed25519: refuse such a key before connecting.
pub fn check_key(key: &Path) -> Result<(), String> {
    let bytes =
        std::fs::read(key).map_err(|e| format!("{ENV}cannot read key {}: {e}", key.display()))?;
    let text = String::from_utf8_lossy(&bytes);
    let needle = b"ssh-ed25519";
    let has = |h: &[u8]| h.windows(needle.len()).any(|w| w == needle);
    let body: String = text.lines().filter(|l| !l.starts_with("-----")).collect();
    if text.trim_start().starts_with("ssh-ed25519") || has(&unb64(&body)) {
        return Err(format!(
            "{ENV}{} is an ed25519 key; the unit's OpenSSH 5.9 cannot use it. Make an ECDSA key: ssh-keygen -t ecdsa -b 256",
            key.display()
        ));
    }
    Ok(())
}

/// Daemon `result` frame -> data, or the finding that names the cause.
pub fn map_result(v: &Value, version: &str) -> Result<Value, Finding> {
    if v["ok"] == true {
        return Ok(json!({"name": v["name"], "version": version, "restart": v["restart"]}));
    }
    let e = v["error"].as_str().unwrap_or("install failed");
    if e.contains("developer mode") {
        return Err(Finding::error(
            "devmode-off",
            &format!("Enable Developer mode on the unit, then push again (daemon: {e})"),
        ));
    }
    Err(Finding::error("install-failed", e))
}

struct Tunnel(Child);
impl Drop for Tunnel {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn free_port() -> Result<u16, String> {
    let l =
        TcpListener::bind("127.0.0.1:0").map_err(|e| format!("{ENV}no free local port: {e}"))?;
    l.local_addr().map(|a| a.port()).map_err(|e| e.to_string())
}

/// One tunnel attempt on a fresh port. ssh's stderr goes to a temp file, shown when it fails.
fn try_tunnel(u: &Unit, skip_wait: bool) -> Result<(Tunnel, u16), String> {
    let port = free_port()?;
    let log = std::env::temp_dir().join(format!(
        "revised-sdk-tunnel-{}-{port}.log",
        std::process::id()
    ));
    let errf = std::fs::File::create(&log).map_err(|e| format!("{ENV}{}: {e}", log.display()))?;
    let child = Command::new("ssh")
        .args(tunnel_args(&u.host, &u.key, port))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(errf)
        .spawn()
        .map_err(|e| format!("{ENV}cannot run `ssh`: {e}"))?;
    let mut t = Tunnel(child);
    let fail = |t: &mut Tunnel, why: String| {
        let _ = t.0.kill();
        let _ = t.0.wait();
        let e = std::fs::read_to_string(&log).unwrap_or_default();
        let _ = std::fs::remove_file(&log);
        format!("{ENV}{why}: {}", e.trim())
    };
    if skip_wait {
        // test seam: no real forward to probe. Wait for the (fake) ssh to say it started, else the
        // child can be killed on drop before it ran at all.
        let t0 = Instant::now();
        while t0.elapsed() < Duration::from_secs(10)
            && std::fs::metadata(&log).map_or(true, |m| m.len() == 0)
        {
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = std::fs::remove_file(&log);
        return Ok((t, port));
    }
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(15) {
        if let Ok(Some(st)) = t.0.try_wait() {
            return Err(fail(
                &mut t,
                format!(
                    "ssh tunnel to the daemon exited ({st}); is the daemon running on the unit?"
                ),
            ));
        }
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            // a stolen port also accepts: ssh must still be alive after the connect
            std::thread::sleep(Duration::from_millis(100));
            if let Ok(Some(st)) = t.0.try_wait() {
                return Err(fail(
                    &mut t,
                    format!("ssh tunnel exited ({st}) after the port opened"),
                ));
            }
            let _ = std::fs::remove_file(&log);
            return Ok((t, port));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Err(fail(
        &mut t,
        "ssh tunnel to the daemon did not come up in 15s".into(),
    ))
}

/// The tunnel, retried once on a fresh port (the free-port probe can lose a race).
fn open_tunnel(u: &Unit, skip_wait: bool) -> Result<(Tunnel, u16), String> {
    try_tunnel(u, skip_wait).or_else(|_| try_tunnel(u, skip_wait))
}

/// `REVISED_SDK_WS_URL` (test seam): honoured only for a loopback host, and always announced.
fn ws_override() -> Result<Option<String>, String> {
    let Ok(url) = std::env::var(WS_URL_ENV) else {
        return Ok(None);
    };
    let host = url.strip_prefix("ws://").map(|r| {
        if let Some(v6) = r.strip_prefix('[') {
            v6.split(']').next().unwrap_or("")
        } else {
            r.split([':', '/']).next().unwrap_or("")
        }
    });
    if !matches!(host, Some("127.0.0.1" | "localhost" | "::1")) {
        return Err(format!("{ENV}{WS_URL_ENV}={url} ignored: only ws:// to a loopback host (127.0.0.1, localhost, ::1) is accepted"));
    }
    eprintln!("warning: {WS_URL_ENV} is set: using {url} instead of the unit's daemon");
    Ok(Some(url))
}

/// Send `install-push` for `remote` and return the daemon's `result` frame.
fn ws_install(url: &str, remote: &str) -> Result<Value, String> {
    let (mut ws, _) =
        tungstenite::connect(url).map_err(|e| format!("{ENV}daemon WS {url}: {e}"))?;
    if let MaybeTlsStream::Plain(s) = ws.get_ref() {
        let _ = s.set_read_timeout(Some(Duration::from_secs(180)));
    }
    let m =
        json!({"control": "store", "action": "install-push", "value": {"path": remote}, "ts": 1});
    ws.send(Message::text(m.to_string()))
        .map_err(|e| format!("{ENV}daemon WS send: {e}"))?;
    loop {
        match ws.read() {
            Ok(Message::Text(t)) => {
                let v: Value = serde_json::from_str(t.as_str()).unwrap_or(Value::Null);
                if v["t"] == "result" && v["op"] == "install-push" {
                    let _ = ws.close(None);
                    return Ok(v);
                }
                if v["t"] == "error" {
                    return Err(format!(
                        "{ENV}daemon: {}",
                        v["error"].as_str().unwrap_or("error")
                    ));
                }
            }
            Ok(_) => {}
            Err(e) => return Err(format!("{ENV}no install result from the daemon: {e}")),
        }
    }
}

/// `--unit`/`--key` flags over `.revised/local.toml`; flags are saved back (after the key check).
fn resolve(rest: &[String], project: &Path, save: bool) -> Result<Unit, String> {
    let val = |f: &str| {
        rest.iter()
            .position(|a| a == f)
            .and_then(|i| rest.get(i + 1))
            .cloned()
    };
    let mut l = local::load(project);
    let (fh, fk) = (val("--unit"), val("--key"));
    let flagged = fh.is_some() || fk.is_some();
    l.unit_host = fh.or(l.unit_host);
    l.unit_key = fk.map(PathBuf::from).or(l.unit_key);
    let (Some(host), Some(key)) = (l.unit_host.clone(), l.unit_key.clone()) else {
        return Err(format!(
            "{ENV}no unit configured: pass --unit cmu@<ip> --key <ecdsa key file> (saved to .revised/local.toml)"
        ));
    };
    if host.starts_with('-') {
        return Err(format!("{ENV}bad --unit value {host:?}"));
    }
    check_key(&key)?;
    if save && flagged {
        local::save(project, &l)
            .map_err(|e| format!("{ENV}cannot save .revised/local.toml: {e}"))?;
    }
    Ok(Unit { host, key })
}

fn push(u: &Unit, project: &Path) -> Result<Value, Vec<Finding>> {
    let env = |m: String| vec![Finding::error("env", &m)];
    let ov = ws_override().map_err(env)?;
    let rvp = build::build(project)?;
    let version = modyaml::load(project)?.version;
    let file = rvp
        .file_name()
        .map_or(String::new(), |f| f.to_string_lossy().into_owned());
    // defence in depth (modyaml already limits name and version): scp's remote path stays unquoted
    // because modern scp speaks SFTP and would take quotes literally
    if !file.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'+' | b'-')) {
        return Err(env(format!("refusing to push {file:?}: unexpected characters in the file name")));
    }
    u.ssh(&format!("mkdir -p {PUSH_DIR}")).map_err(env)?;
    u.scp(
        &rvp.to_string_lossy(),
        &u.remote(&format!("{PUSH_DIR}/{file}")),
    )
    .map_err(env)?;
    let (_tunnel, port) = open_tunnel(u, ov.is_some()).map_err(env)?;
    let url = ov.unwrap_or_else(|| format!("ws://127.0.0.1:{port}"));
    let r = ws_install(&url, &format!("{PUSH_DIR}/{file}")).map_err(env)?;
    map_result(&r, &version).map_err(|f| vec![f])
}

/// Returns the exit code. `rest` = the command words (`push ...` / `unit <sub> ...`).
pub fn command(rest: &[String], json: bool, project: &Path) -> i32 {
    let is_push = rest[0] == "push";
    let cmd = if is_push {
        "push".to_string()
    } else {
        format!("unit {}", rest.get(1).map_or("", |s| s))
    };
    // positional args after the sub-command, minus the --unit/--key pairs
    let mut pos = Vec::new();
    let mut it = rest.iter().skip(if is_push { 1 } else { 2 });
    while let Some(a) = it.next() {
        match a.as_str() {
            "--unit" | "--key" => {
                it.next();
            }
            _ => pos.push(a.clone()),
        }
    }
    let usage = "usage: revised-sdk push [--unit cmu@<ip>] [--key <file>] | unit logs [n] | unit shot <png> | unit eval '<js>'";
    let sub = rest.get(1).map(String::as_str);
    let shape_ok = is_push && pos.is_empty()
        || matches!(
            (sub, pos.len()),
            (Some("logs"), 0 | 1) | (Some("shot" | "eval"), 1)
        );
    if !shape_ok {
        return report::env_error_in(json, &cmd, usage);
    }
    let u = match resolve(rest, project, is_push) {
        Ok(u) => u,
        Err(m) => return report::env_error_in(json, &cmd, m.strip_prefix(ENV).unwrap_or(&m)),
    };
    let r: Result<(String, Value), Vec<Finding>> = if is_push {
        push(&u, project).map(|d| {
            (
                format!(
                    "pushed {} {}",
                    d["name"].as_str().unwrap_or(""),
                    d["version"].as_str().unwrap_or("")
                ),
                d,
            )
        })
    } else {
        let arg = pos.first().map_or("", |s| s);
        let r = match sub {
            Some("logs") => logs(&u, project, arg.parse().unwrap_or(30))
                .map(|l| (l.join("\n"), json!({"lines": l}))),
            Some("shot") => shot(&u, arg).map(|p| (p.clone(), json!({"path": p}))),
            _ => eval(&u, arg, EVAL_SECS),
        };
        r.map_err(|m| match m.strip_prefix(ENV) {
            Some(e) => vec![Finding::error("env", e)],
            None => vec![Finding::error(sub.unwrap_or("unit"), &m)],
        })
    };
    match r {
        Ok((_, data)) if json => report::Report::new(&cmd, data, vec![]).emit(true),
        Ok((text, _)) => {
            println!("{text}");
            0
        }
        Err(f) if f.iter().all(|x| x.rule == "env") => {
            let m = f
                .iter()
                .map(|x| x.msg.strip_prefix(ENV).unwrap_or(&x.msg))
                .collect::<Vec<_>>()
                .join("\n");
            report::env_error_in(json, &cmd, &m)
        }
        Err(f) => report::Report::new(&cmd, json!({}), f).emit(json),
    }
}

/// Last `n` unit log lines that mention this package or an `[rvp-sdk` tag.
fn logs(u: &Unit, project: &Path, n: usize) -> Result<Vec<String>, String> {
    let name = modyaml::load(project).map(|m| m.name).unwrap_or_default();
    let out = u.ssh(&format!("tail -n {n} {UNIT_LOG}"))?;
    Ok(out
        .lines()
        .filter(|l| l.contains("[rvp-sdk") || (!name.is_empty() && l.contains(&name)))
        .map(String::from)
        .collect())
}

fn shot(u: &Unit, dest: &str) -> Result<String, String> {
    if Path::new(dest).extension().is_none_or(|x| x != "png") {
        return Err(format!("shot target must be a .png file, got {dest:?}"));
    }
    // scp reads `host:path` as a remote target and `-x` as an option
    if dest.starts_with('-') || dest.contains(':') {
        return Err(format!("{ENV}shot target must not start with `-` or contain `:`, got {dest:?}"));
    }
    // the tool needs the HMI's Wayland env and always writes ./wayland-screenshot.png
    u.ssh(&format!(
        "cd /tmp && rm -f wayland-screenshot.png && XDG_RUNTIME_DIR=/tmp WAYLAND_DISPLAY=wayland-0 \
         /usr/bin/screenshot >/dev/null 2>&1; mv -f /tmp/wayland-screenshot.png {REMOTE_SHOT}"
    ))?;
    u.scp(&u.remote(REMOTE_SHOT), dest)?;
    Ok(dest.to_string())
}

fn eval(u: &Unit, js: &str, secs: u64) -> Result<(String, Value), String> {
    // a page reload in the HMI reboots the whole unit ~1 min later (bench 2026-11-16)
    if js.contains(".reload(") {
        return Err("refused: reloading the HMI page reboots the unit; use SafeReboot".into());
    }
    let t0 = Instant::now();
    let left = || secs.saturating_sub(t0.elapsed().as_secs()).max(1);
    let no_reply = || {
        format!("no reply from the page within {secs}s (is the DEV eval bridge installed and the HMI running?)")
    };
    // every ssh call is bounded by what is left of the deadline; a stuck one reads as "no reply"
    let ssh = |cmd: &str| {
        u.ssh_for(cmd, left())
            .map_err(|e| if e.contains(TIMED_OUT) { no_reply() } else { e })
    };
    // the daemon logs eval replies to its own log; the unit log is kept as a second source
    let grep = format!("grep -hF '\"control\":\"eval\"' {DAEMON_LOG} {UNIT_LOG} 2>/dev/null || true");
    // grep exits 1 while there is no match: `|| true` keeps "0 replies" from reading as a failure
    let before = count_replies(&ssh(&grep)?);
    // the FIFO open blocks without a reader: BusyBox `timeout -t` ends it (GNU `timeout 3` would not parse)
    ssh(&format!(
        "timeout -t 3 sh -c 'printf \"%s\\n\" \"$1\" > /tmp/rvp-ctl' sh '{}'",
        eval_line(js)
    ))?;
    while t0.elapsed() < Duration::from_secs(secs) {
        if let Some(r) = reply(&ssh(&grep)?, before) {
            let v = r.map_err(|m| format!("script error: {m}"))?;
            return Ok((v.clone(), json!({"result": v})));
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    Err(no_reply())
}
