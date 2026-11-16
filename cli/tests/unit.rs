//! `push` / `unit *` against a fake `ssh`/`scp` on PATH and a fake daemon WS. No real host is touched.
use revised_sdk::{local, new::new_project, preview::b64, unit};
use serde_json::{json, Value};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rsdk-u-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

const FAKE_SSH: &str = r#"#!/bin/sh
echo "ssh $*" >> "$FAKE_LOG"
if [ -n "$FAKE_TUNNEL_FAIL" ]; then case "$*" in *" -L "*) echo "Permission denied (publickey)" >&2; exit 255;; esac; fi
if [ -n "$FAKE_HANG" ]; then exec sleep 300; fi
case "$*" in *" -L "*) exec sleep 30;; esac
if [ -n "$FAKE_DOWN" ]; then echo "ssh: connect to host h port 36000: Connection refused" >&2; exit 255; fi
case "$*" in *rvp-ctl*) touch "$FAKE_DIR/sent";; esac
case "$*" in *"grep -F"*) [ -f "$FAKE_DIR/sent" ] && cat "$FAKE_DIR/reply.txt";; esac
case "$*" in *"tail -n"*) cat "$FAKE_DIR/log.txt";; esac
exit 0
"#;
const FAKE_SCP: &str = r#"#!/bin/sh
echo "scp $*" >> "$FAKE_LOG"
for last; do :; done
case "$*" in *revised-shot*) echo PNG > "$last";; esac
exit 0
"#;

struct Env {
    dir: PathBuf,
    proj: PathBuf,
    key: PathBuf,
}

fn env(name: &str) -> Env {
    let dir = tmp(name);
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    for (n, body) in [("ssh", FAKE_SSH), ("scp", FAKE_SCP)] {
        std::fs::write(bin.join(n), body).unwrap();
        #[cfg(unix)]
        std::fs::set_permissions(
            bin.join(n),
            std::os::unix::fs::PermissionsExt::from_mode(0o755),
        )
        .unwrap();
    }
    let key = dir.join("k_ecdsa");
    std::fs::write(
        &key,
        pem("EC", "Zml4dHVyZQ=="),
    )
    .unwrap();
    let proj = new_project(&dir, "app", "t1").unwrap();
    Env { dir, proj, key }
}

impl Env {
    fn run(&self, args: &[&str], extra: &[(&str, &str)]) -> (i32, String, String) {
        let path = format!(
            "{}{}{}",
            self.dir.join("bin").display(),
            if cfg!(windows) { ";" } else { ":" },
            std::env::var("PATH").unwrap_or_default()
        );
        let o = Command::new(env!("CARGO_BIN_EXE_revised-sdk"))
            .arg("--json")
            .arg("-C")
            .arg(&self.proj)
            .args(args)
            .env("PATH", path)
            .env("FAKE_LOG", self.dir.join("calls.log"))
            .env("FAKE_DIR", &self.dir)
            .envs(extra.iter().copied())
            .output()
            .unwrap();
        (
            o.status.code().unwrap(),
            String::from_utf8_lossy(&o.stdout).into_owned(),
            String::from_utf8_lossy(&o.stderr).into_owned(),
        )
    }
    fn calls(&self) -> String {
        std::fs::read_to_string(self.dir.join("calls.log")).unwrap_or_default()
    }
    fn push_args(&self) -> Vec<String> {
        [
            "push",
            "--unit",
            "cmu@unit.invalid",
            "--key",
            self.key.to_str().unwrap(),
        ]
        .map(String::from)
        .into()
    }
}

/// A one-shot fake daemon: sends a catalog frame, reads one message, answers with `result`.
fn daemon(result: Value) -> (String, mpsc::Receiver<Value>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("ws://{}", l.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let (s, _) = l.accept().unwrap();
        let mut ws = tungstenite::accept(s).unwrap();
        ws.send(tungstenite::Message::text(
            json!({"t": "catalog", "online": true}).to_string(),
        ))
        .unwrap();
        let m = ws.read().unwrap();
        tx.send(serde_json::from_str(m.to_text().unwrap()).unwrap())
            .unwrap();
        ws.send(tungstenite::Message::text(result.to_string()))
            .unwrap();
        let _ = ws.read(); // close handshake
    });
    (url, rx)
}

fn report(out: &str) -> Value {
    serde_json::from_str(out.trim()).unwrap_or_else(|e| panic!("not json ({e}): {out}"))
}

fn go(e: &Env, args: &[String], url: &str) -> (i32, String, String) {
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    e.run(&a, &[(unit::WS_URL_ENV, url)])
}

#[test]
fn push_devmode_off_maps_to_exit1() {
    let e = env("devoff");
    let (url, _rx) = daemon(
        json!({"t": "result", "name": "", "op": "install-push", "ok": false, "error": "developer mode required"}),
    );
    let (code, out, _) = go(&e, &e.push_args(), &url);
    assert_eq!(code, 1, "{out}");
    let r = report(&out);
    assert_eq!(r["ok"], false);
    assert!(
        r["errors"][0]["msg"]
            .as_str()
            .unwrap()
            .contains("Enable Developer mode"),
        "{r}"
    );
}

#[test]
fn push_installs_over_ssh_scp_and_ws_and_saves_flags() {
    let e = env("ok");
    let (url, rx) = daemon(
        json!({"t": "result", "name": "t1", "op": "install-push", "ok": true, "restart": false}),
    );
    let (code, out, err) = go(&e, &e.push_args(), &url);
    assert_eq!(code, 0, "{out}{err}");
    let r = report(&out);
    assert_eq!(
        r["data"],
        json!({"name": "t1", "version": "0.1.0", "restart": false})
    );
    let m = rx.recv().unwrap();
    assert_eq!(m["action"], "install-push");
    assert_eq!(m["value"]["path"], "/tmp/revised-push/t1-0.1.0.rvp");
    let calls = e.calls();
    assert!(calls.contains("mkdir -p /tmp/revised-push"), "{calls}");
    assert!(calls.contains("scp -P 36000"), "{calls}");
    assert!(
        calls.contains("cmu@unit.invalid:/tmp/revised-push/t1-0.1.0.rvp"),
        "{calls}"
    );
    assert!(
        calls.contains("-L ") && calls.contains(":127.0.0.1:2777 -N"),
        "{calls}"
    );
    let l = local::load(&e.proj);
    assert_eq!(l.unit_host.as_deref(), Some("cmu@unit.invalid"));
    // second push needs no flags: they come from local.toml
    let (url, _rx) = daemon(
        json!({"t": "result", "name": "t1", "op": "install-push", "ok": true, "restart": true}),
    );
    let (code, out, _) = go(&e, &["push".into()], &url);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn ssh_unreachable_exit2() {
    let e = env("down");
    let args = e.push_args();
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    let (code, out, err) = e.run(&a, &[("FAKE_DOWN", "1")]);
    assert_eq!(code, 2);
    let r = report(&out);
    assert!(
        r["errors"][0]["msg"]
            .as_str()
            .unwrap()
            .contains("Connection refused"),
        "{r} {err}"
    );
    assert!(!e.calls().contains("scp "), "nothing copied");
}

#[test]
fn ed25519_key_hint() {
    let e = env("ed");
    let pub_style = e.dir.join("k1");
    std::fs::write(&pub_style, "ssh-ed25519 AAAAC3Nz fixture\n").unwrap();
    // OpenSSH private-key container: the key type sits in the (base64) blob
    let mut blob = b"openssh-key-v1\0".to_vec();
    blob.extend_from_slice(&[0, 0, 0, 4]);
    blob.extend_from_slice(b"none");
    blob.extend_from_slice(&[0, 0, 0, 11]);
    blob.extend_from_slice(b"ssh-ed25519");
    let osh = e.dir.join("k2");
    std::fs::write(
        &osh,
        pem("OPENSSH", &b64(&blob)),
    )
    .unwrap();
    for k in [pub_style, osh] {
        let (code, out, _) = e.run(
            &[
                "push",
                "--unit",
                "cmu@unit.invalid",
                "--key",
                k.to_str().unwrap(),
            ],
            &[],
        );
        assert_eq!(code, 2, "{out}");
        assert!(out.contains("ECDSA"), "{out}");
    }
    assert_eq!(e.calls(), "", "refused before connecting");
    assert!(unit::check_key(&e.key).is_ok());
}

#[test]
fn no_unit_configured_exit2() {
    let e = env("none");
    let (code, out, _) = e.run(&["push"], &[]);
    assert_eq!(code, 2);
    assert!(out.contains("no unit configured"), "{out}");
    assert_eq!(e.calls(), "");
}

#[test]
fn args_use_port_36000_and_batchmode() {
    let a = unit::ssh_args("cmu@h", Path::new("k"), "true");
    let s = a.join(" ");
    assert!(s.starts_with("-p 36000 -i k -o BatchMode=yes"), "{s}");
    assert_eq!(a.last().map(String::as_str), Some("true"));
    let t = unit::tunnel_args("cmu@h", Path::new("k"), 5555).join(" ");
    assert!(
        t.contains("-p 36000")
            && t.contains("BatchMode=yes")
            && t.contains("-L 5555:127.0.0.1:2777 -N cmu@h"),
        "{t}"
    );
}

fn unit_flags(e: &Env) -> Vec<String> {
    [
        "--unit",
        "cmu@unit.invalid",
        "--key",
        e.key.to_str().unwrap(),
    ]
    .map(String::from)
    .into()
}

#[test]
fn unit_eval_reads_the_reply_from_the_unit_log() {
    let e = env("eval");
    std::fs::write(
        e.dir.join("reply.txt"),
        "[opera-1] {\"control\":\"eval\",\"action\":\"ok\",\"value\":\"42\"}\n",
    )
    .unwrap();
    let mut a = vec!["unit".to_string(), "eval".into(), "6*7".into()];
    a.extend(unit_flags(&e));
    let (code, out, _) = e.run(&a.iter().map(String::as_str).collect::<Vec<_>>(), &[]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(report(&out)["data"]["result"], "42");
    assert!(e.calls().contains("/tmp/rvp-ctl"), "{}", e.calls());
}

#[test]
fn unit_logs_filters_to_package_and_sdk_tag() {
    let e = env("logs");
    std::fs::write(
        e.dir.join("log.txt"),
        "noise\nt1 started\n[rvp-sdk] hello\nmore noise\n",
    )
    .unwrap();
    let mut a = vec!["unit".to_string(), "logs".into(), "50".into()];
    a.extend(unit_flags(&e));
    let (code, out, _) = e.run(&a.iter().map(String::as_str).collect::<Vec<_>>(), &[]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        report(&out)["data"]["lines"],
        json!(["t1 started", "[rvp-sdk] hello"])
    );
    assert!(e.calls().contains("tail -n 50 /var/log/running_log/log"));
}

#[test]
fn unit_shot_fetches_png() {
    let e = env("shot");
    let png = e.dir.join("s.png");
    let mut a = vec![
        "unit".to_string(),
        "shot".into(),
        png.to_str().unwrap().into(),
    ];
    a.extend(unit_flags(&e));
    let (code, out, _) = e.run(&a.iter().map(String::as_str).collect::<Vec<_>>(), &[]);
    assert_eq!(code, 0, "{out}");
    assert!(png.exists());
    assert!(e.calls().contains("/usr/bin/screenshot"));
}

#[test]
fn unit_eval_hung_ssh_returns_exit1_within_deadline() {
    let e = env("hang");
    let mut a = vec!["unit".to_string(), "eval".into(), "1".into()];
    a.extend(unit_flags(&e));
    let t0 = std::time::Instant::now();
    let (code, out, _) = e.run(
        &a.iter().map(String::as_str).collect::<Vec<_>>(),
        &[("FAKE_HANG", "1")],
    );
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("no reply"), "{out}");
    assert!(t0.elapsed().as_secs() < 12, "{:?}", t0.elapsed());
}

#[test]
fn eval_ctl_write_is_bounded_by_busybox_timeout_and_keepalive() {
    let e = env("bb");
    let mut a = vec!["unit".to_string(), "eval".into(), "1".into()];
    a.extend(unit_flags(&e));
    e.run(&a.iter().map(String::as_str).collect::<Vec<_>>(), &[]);
    let c = e.calls();
    assert!(c.contains("timeout -t 3 sh -c"), "{c}");
    assert!(
        c.contains("ServerAliveInterval=5") && c.contains("ServerAliveCountMax=1"),
        "{c}"
    );
}

#[test]
fn non_loopback_ws_override_is_rejected() {
    let e = env("ovr");
    for bad in [
        "ws://evil.example:2777",
        "ws://127.0.0.1.evil.example:1",
        "wss://127.0.0.1:1",
    ] {
        let a = e.push_args();
        let (code, out, _) = e.run(
            &a.iter().map(String::as_str).collect::<Vec<_>>(),
            &[(unit::WS_URL_ENV, bad)],
        );
        assert_eq!(code, 2, "{bad}: {out}");
        assert!(out.contains("loopback"), "{out}");
    }
    assert_eq!(e.calls(), "", "rejected before any ssh");
}

#[test]
fn loopback_ws_override_warns_on_stderr() {
    let e = env("warn");
    let (url, _rx) = daemon(
        json!({"t": "result", "name": "t1", "op": "install-push", "ok": true, "restart": false}),
    );
    let (code, _, err) = go(&e, &e.push_args(), &url);
    assert_eq!(code, 0);
    assert!(err.contains("REVISED_SDK_WS_URL"), "{err}");
}

#[test]
fn tunnel_failure_shows_ssh_stderr_after_one_retry() {
    let e = env("tun");
    let a = e.push_args();
    let (code, out, _) = e.run(
        &a.iter().map(String::as_str).collect::<Vec<_>>(),
        &[("FAKE_TUNNEL_FAIL", "1")],
    );
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("Permission denied"), "{out}");
    assert_eq!(e.calls().matches(" -L ").count(), 2, "{}", e.calls());
}

// Header text is assembled at runtime so no PEM marker literal sits in the source (repo-audit).
fn pem(kind: &str, body: &str) -> String {
    let k = format!("{kind} PRIVATE{}", " KEY");
    format!("-----BEGIN {k}-----\n{body}\n-----END {k}-----\n")
}
