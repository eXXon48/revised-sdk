use revised_sdk::{build, engine, new, report, setup};
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
    if cmd == "new" || cmd == "build" {
        return project_command(&rest, json, &project);
    }
    if std::env::var_os("REVISED_SDK_IN_CONTAINER").is_none() {
        // container-side commands re-exec in the image
        return match engine::detect(eng.as_deref()).and_then(|e| {
            engine::run_in_image(&e, &project, None, &without_host_flags(&args))
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
