use revised_sdk::engine::{self, Engine};
use revised_sdk::preview::*;
use std::path::PathBuf;
use std::process::Command;

// ---- pure parts (always run) ----

#[test]
fn key_names_map_to_stub_keycodes() {
    let want = [
        ("up", 38),
        ("down", 40),
        ("left", 37),
        ("right", 39),
        ("enter", 13),
        ("back", 8),
        ("home", 72),
    ];
    for (n, c) in want {
        assert_eq!(keycode(n), Some(c), "{n}");
    }
    assert_eq!(keycode("escape"), None);
}

#[test]
fn base64_matches_rfc4648() {
    assert_eq!(b64(b""), "");
    assert_eq!(b64(b"f"), "Zg==");
    assert_eq!(b64(b"fo"), "Zm8=");
    assert_eq!(b64(b"foo"), "Zm9v");
    assert_eq!(b64(b"1+1"), "MSsx");
}

#[test]
fn eval_line_is_ctl_js_eval_atob() {
    assert_eq!(eval_line("1+1"), "js:eval(atob(\"MSsx\"))");
}

#[test]
fn eval_reply_is_the_nth_new_log_line() {
    let log = "x\n[1]   event    1.2.3.4:5  {\"control\":\"env\",\"action\":\"ua\"}\n\
               [2]   event    1.2.3.4:5  {\"control\":\"eval\",\"action\":\"ok\",\"value\":\"old\",\"ts\":1}\n\
               [3]   event    1.2.3.4:5  {\"control\":\"eval\",\"action\":\"ok\",\"value\":\"2\",\"ts\":2}\n";
    assert_eq!(count_replies(log), 2);
    assert_eq!(reply(log, 1), Some(Ok("2".into())));
    assert_eq!(reply(log, 2), None);
    let err =
        "[3] event a {\"control\":\"eval\",\"action\":\"err\",\"value\":\"ReferenceError: x\"}\n";
    assert_eq!(reply(err, 0), Some(Err("ReferenceError: x".into())));
}

#[test]
fn run_args_bind_loopback_only_and_mount_project_and_opera() {
    let a = run_args("revised-sdk:0.1.0", "C:/p q/proj", "dist/a-0.1.0.rvp", true).join(" ");
    assert!(a.contains("--name revised-sdk-preview"), "{a}");
    assert!(
        a.contains("-p 127.0.0.1:2777:2777") && a.contains("-p 127.0.0.1:8750:8750"),
        "{a}"
    );
    assert!(!a.contains("0.0.0.0"), "{a}");
    assert!(a.contains("-v C:/p q/proj:/work"), "{a}");
    assert!(a.contains("-v revised-sdk-opera:/opera:ro"), "{a}");
    assert!(a.contains("-e RVP=/work/dist/a-0.1.0.rvp"), "{a}");
    assert!(a.contains("START_OPERA=1"), "{a}");
    assert!(run_args("t", "/p", "d/a.rvp", false)
        .join(" ")
        .contains("START_OPERA=0"));
}

#[test]
fn shot_path_must_stay_inside_the_project() {
    assert!(shot_target("out/s.png").is_ok());
    assert!(shot_target("../s.png").is_err());
    assert!(shot_target("/tmp/s.png").is_err());
    assert!(shot_target("s.txt").is_err());
    assert!(shot_target("..\\x.png").is_err());
    assert!(shot_target("a\\..\\..\\x.png").is_err());
    assert!(shot_target("\\x.png").is_err());
    assert_eq!(shot_target("a\\b.png").unwrap(), "a/b.png");
}

// ---- integration (real image + engine; REVISED_SDK_IT=1) ----

fn it() -> Option<Engine> {
    std::env::var_os("REVISED_SDK_IT")?;
    Some(engine::detect(None).expect("engine"))
}

fn project(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rsdk-pv-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    revised_sdk::new::new_project(&d, "app", "pvapp").unwrap()
}

fn count_containers(e: &Engine) -> usize {
    let o = Command::new(&e.bin)
        .args(["ps", "-a", "-q", "--filter", &format!("name=^{CONTAINER}$")])
        .output()
        .unwrap();
    String::from_utf8_lossy(&o.stdout).lines().count()
}

// Opera needs its license accepted by the developer: those tests run only with
// REVISED_SDK_IT_OPERA=1 (= the person running the tests accepts it).
fn opera() -> bool {
    std::env::var_os("REVISED_SDK_IT_OPERA").is_some()
}

#[test]
fn up_down_up_idempotent() {
    let Some(e) = it() else { return };
    let p = project("idem");
    let o = UpOpts { opera: false };
    up(&e, &p, &o).unwrap();
    let first = up(&e, &p, &o).unwrap();
    assert_eq!(first, URL);
    assert_eq!(count_containers(&e), 1);
    down(&e).unwrap();
    assert_eq!(count_containers(&e), 0);
    down(&e).unwrap(); // down twice is fine
}

#[test]
fn eval_timeout_is_exit_1() {
    let Some(e) = it() else { return };
    let p = project("timeout");
    // Start the container by hand on other host ports: a browser the developer left open on
    // 127.0.0.1:2777/:8750 cannot reach it, so nobody can answer and the silence path always runs.
    let rvp = revised_sdk::build::build(&p).unwrap();
    let rel = rvp
        .strip_prefix(&p)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    let mut args = run_args(
        &revised_sdk::engine::image_tag(),
        &revised_sdk::engine::abs_host_path(&p).unwrap(),
        &rel,
        false,
    );
    for a in args.iter_mut() {
        *a = a
            .replace("127.0.0.1:2777:", "127.0.0.1:12777:")
            .replace("127.0.0.1:8750:", "127.0.0.1:18750:");
    }
    args.splice(2..2, ["--log-driver".to_string(), "k8s-file".to_string()]);
    let _ = Command::new(&e.bin).args(["rm", "-f", CONTAINER]).output();
    let o = Command::new(&e.bin).args(&args).output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let mut ready = false;
    for _ in 0..120 {
        let l = Command::new(&e.bin)
            .args(["logs", CONTAINER])
            .output()
            .unwrap();
        let t = format!(
            "{}{}",
            String::from_utf8_lossy(&l.stdout),
            String::from_utf8_lossy(&l.stderr)
        );
        if t.contains("[preview] installed") {
            ready = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    assert!(ready, "container never became ready");
    // REVISED_SDK_BIN: the CLI binary to drive (when the tests run outside the build container)
    let bin = std::env::var("REVISED_SDK_BIN")
        .unwrap_or_else(|_| env!("CARGO_BIN_EXE_revised-sdk").into());
    let out = Command::new(bin)
        .arg("-C")
        .arg(&p)
        .args(["preview", "eval", "1+1"])
        .output()
        .unwrap();
    let _ = down(&e);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(text.contains("no reply"), "{text}");
}

#[test]
fn shot_writes_png_800x480() {
    let Some(e) = it() else { return };
    let p = project("shot");
    up(&e, &p, &UpOpts { opera: opera() }).unwrap();
    shot(&e, &p, "shots/a.png").unwrap();
    let b = std::fs::read(p.join("shots/a.png")).unwrap();
    assert_eq!(&b[..8], b"\x89PNG\r\n\x1a\n");
    let (w, h) = (
        u32::from_be_bytes(b[16..20].try_into().unwrap()),
        u32::from_be_bytes(b[20..24].try_into().unwrap()),
    );
    assert_eq!((w, h), (800, 480));
    let _ = down(&e);
}

// A browser the developer left open on :8750 can answer too: the reply must come from the container Opera (X11).
fn assert_container_opera(e: &Engine) {
    let ua = eval(e, "navigator.userAgent").unwrap();
    assert!(
        ua.contains("X11"),
        "reply is not from the container Opera: {ua}"
    );
}

#[test]
fn eval_returns_value() {
    let Some(e) = it().filter(|_| opera()) else {
        return;
    };
    up(&e, &project("eval"), &UpOpts { opera: true }).unwrap();
    assert_eq!(eval(&e, "1+1").unwrap(), "2");
    assert_container_opera(&e);
    let _ = down(&e);
}

#[test]
fn app_visible_after_up() {
    let Some(e) = it().filter(|_| opera()) else {
        return;
    };
    up(&e, &project("app"), &UpOpts { opera: true }).unwrap();
    assert_container_opera(&e);
    eval(&e, "simGo('Applications')").unwrap();
    // rows: 4 stock, then the Revised Store row, then pvapp
    for _ in 0..5 {
        key(&e, "down").unwrap();
    }
    key(&e, "enter").unwrap();
    let mut seen = false;
    for _ in 0..10 {
        if eval(&e, "document.body.innerHTML.indexOf('Hello from pvapp')>=0").unwrap() == "true" {
            seen = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    assert!(seen, "app overlay never showed");
    let _ = down(&e);
}

#[test]
fn license_help_names_the_detected_engine_and_real_image() {
    let d = std::env::temp_dir().join(format!("rsdk-lic-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("docker"), "").unwrap();
    std::fs::write(d.join("docker.exe"), "").unwrap();
    let bin = std::env::var("REVISED_SDK_BIN")
        .unwrap_or_else(|_| env!("CARGO_BIN_EXE_revised-sdk").into());
    let o = Command::new(bin)
        .args(["preview", "up"])
        .env("PATH", &d)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&d);
    let t = String::from_utf8_lossy(&o.stderr).to_string() + &String::from_utf8_lossy(&o.stdout);
    assert_eq!(o.status.code(), Some(2), "{t}");
    assert!(t.contains("`docker run --rm -v revised-sdk-opera:/opera:ro revised-sdk:"), "{t}");
    assert!(!t.contains("<version>") && !t.contains("podman"), "{t}");
}

#[test]
fn ctl_write_without_a_reader_returns_in_bounded_time() {
    let Some(e) = it() else { return };
    let p = project("fifo");
    up(&e, &p, &UpOpts { opera: false }).unwrap();
    // the daemon is the FIFO reader: kill it and the open blocks
    let kill = "for p in /proc/[0-9]*; do grep -q '^revised-daemon' $p/comm && kill ${p#/proc/}; done; sleep 1";
    let _ = Command::new(&e.bin).args(["exec", CONTAINER, "sh", "-c", kill]).output().unwrap();
    let t0 = std::time::Instant::now();
    let r = key(&e, "up");
    let _ = down(&e);
    assert!(t0.elapsed().as_secs() < 15, "{:?}", t0.elapsed());
    assert!(r.unwrap_err().contains("no reply"), "ctl must time out");
}
