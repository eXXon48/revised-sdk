use revised_sdk::{baseline, build, engine, lint, local, new, preview, report, setup};
use serde_json::json;
use std::path::PathBuf;

fn main() {
    std::process::exit(run(std::env::args().skip(1).collect()));
}

fn run(args: Vec<String>) -> i32 {
    let (mut json, mut eng, mut dir) = (false, None, None);
    let mut rest = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--json" => json = true,
            "--engine" => eng = it.next().cloned(),
            "-C" => dir = it.next().map(PathBuf::from),
            _ => rest.push(a.clone()),
        }
    }
    let Some(cmd) = rest.first().cloned() else {
        return report::env_error_in(
            json,
            "revised-sdk",
            "usage: revised-sdk [--json] [--engine podman|docker] [-C dir] <command>",
        );
    };
    if cmd == "setup" || cmd == "opera" {
        return host_command(&rest, json, eng.as_deref());
    }
    let project = dir.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    if cmd == "preview" {
        return preview_command(&rest, json, eng.as_deref(), &project);
    }
    if cmd == "baseline" {
        return match rest.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
            ["baseline", "check", d] => baseline::check_cmd(&project, std::path::Path::new(d), baseline::LIST).emit(json),
            _ => report::env_error_in(json, "baseline", "usage: revised-sdk baseline check <dir>"),
        };
    }
    if cmd == "new" || cmd == "build" || cmd == "lint" {
        return project_command(&rest, json, &project);
    }
    if std::env::var_os("REVISED_SDK_IN_CONTAINER").is_none() {
        // container-side commands re-exec in the image
        return match engine::detect(eng.as_deref()).and_then(|e| {
            engine::run_in_image(&e, &project, local::load(&project).baseline.as_deref(), &without_host_flags(&args))
        }) {
            Ok(code) => code,
            Err(m) => report::env_error_in(json, &cmd, &m),
        };
    }
    report::env_error_in(json, &cmd, "not implemented")
}

/// `-C`/`--engine` are host concerns (the project is mounted at /work); forward the rest.
fn without_host_flags(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--engine" | "-C" => {
                it.next();
            }
            _ => out.push(a.clone()),
        }
    }
    out
}

/// Commands that run on the host (they manage the image itself).
fn host_command(rest: &[String], json: bool, eng: Option<&str>) -> i32 {
    let val = |flag: &str| rest.iter().position(|a| a == flag).and_then(|i| rest.get(i + 1)).map(PathBuf::from);
    let has = |flag: &str| rest.iter().any(|a| a == flag);
    let engine = engine::detect(eng).ok();
    if rest[0] == "setup" {
        let (store, sdk) = (val("--store"), val("--sdk"));
        return setup::setup(&setup::SetupOpts {
            store: store.as_deref(),
            sdk: sdk.as_deref(),
            engine: engine.as_ref(),
            dry_run: has("--dry-run"),
            json,
        });
    }
    if rest.get(1).map(String::as_str) != Some("install") {
        return report::env_error_in(json, "opera", "usage: revised-sdk opera install [--host|--image]");
    }
    let os = std::env::consts::OS;
    let dir = match setup::host_dir(os, &|k| std::env::var(k).ok()) {
        Ok(d) => d,
        Err(m) => return report::env_error_in(json, "opera install", &m),
    };
    setup::opera_install(&setup::OperaOpts {
        manifest: setup::OPERA_MANIFEST,
        os,
        image: !has("--host"),
        host_dir: dir,
        engine: engine.as_ref(),
        json,
        fetch: &setup::curl_fetch,
    })
}

/// Commands that read and write the project on the host (no container needed).
fn project_command(rest: &[String], json: bool, project: &std::path::Path) -> i32 {
    let (cmd, result) = match rest.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["new", tpl, name] => ("new", new::new_project(project, tpl, name)),
        ["new", ..] => return report::env_error_in(json, "new", "usage: revised-sdk new <template> <name>"),
        ["build"] => ("build", build::build(project)),
        ["lint"] => return report::Report::new("lint", json!({}), lint::run(project)).emit(json),
        _ => return report::env_error_in(json, "build", "usage: revised-sdk build"),
    };
    match result {
        Ok(p) => {
            let shown = p.strip_prefix(project).unwrap_or(&p).to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/");
            report::Report::new(cmd, json!({"path": shown}), vec![]).emit(json)
        }
        Err(f) => report::Report::new(cmd, json!({}), f).emit(json),
    }
}

const OPERA_LICENSE_HELP: &str = "Opera 12.10 comes under its own license. Read it with \
`podman run --rm -v revised-sdk-opera:/opera:ro revised-sdk:<version> sh -c 'cat /opera/*/LICENSE'`, \
then add --accept-opera-license to accept it on this machine";

/// `preview up|down|url|eval|shot|logs|key`: drives the preview container from the host.
fn preview_command(rest: &[String], json: bool, eng: Option<&str>, project: &std::path::Path) -> i32 {
    const USAGE: &str = "usage: revised-sdk preview up [--accept-opera-license] | down | url | eval '<js>' | shot <file.png> | logs [n] | key <name>";
    let sub = rest.get(1).map(String::as_str).unwrap_or("");
    let cmd = format!("preview {sub}");
    let arg = rest.get(2).map(String::as_str);
    let accept = rest.iter().any(|a| a == "--accept-opera-license");
    if !matches!(sub, "up" | "down" | "url" | "eval" | "shot" | "logs" | "key") || (arg.is_none() && matches!(sub, "eval" | "shot" | "key")) {
        return report::env_error_in(json, "preview", USAGE);
    }
    if sub == "up" && !accept {
        return report::env_error_in(json, &cmd, OPERA_LICENSE_HELP);
    }
    let e = match engine::detect(eng) {
        Ok(e) => e,
        Err(m) => return report::env_error_in(json, &cmd, &m),
    };
    let arg = arg.unwrap_or("");
    // (text for plain mode, data for --json)
    let r: Result<(String, serde_json::Value), String> = match sub {
        "up" => preview::up(&e, project, &preview::UpOpts { opera: true }).map(|u| (u.clone(), json!({"url": u}))),
        "down" => preview::down(&e).map(|_| ("preview down".into(), json!({}))),
        "url" => preview::url(&e).map(|u| (u.clone(), json!({"url": u}))),
        "eval" => preview::eval(&e, arg).map(|v| (v.clone(), json!({"result": v}))),
        "shot" => preview::shot(&e, project, arg).map(|p| {
            let p = p.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/");
            (p.clone(), json!({"path": p}))
        }),
        "logs" => preview::logs(&e, arg.parse().unwrap_or(30)).map(|l| (l.join("\n"), json!({"lines": l}))),
        _ => preview::key(&e, arg).map(|c| (format!("key {arg} ({c})"), json!({"key": arg, "code": c}))),
    };
    match r {
        Ok((text, data)) => {
            if json {
                report::Report::new(&cmd, data, vec![]).emit(true)
            } else {
                println!("{text}");
                0
            }
        }
        Err(m) => match m.strip_prefix(preview::ENV) {
            Some(env) => report::env_error_in(json, &cmd, env),
            None => report::Report::new(&cmd, json!({}), vec![report::Finding::error(sub, &m)]).emit(json),
        },
    }
}
