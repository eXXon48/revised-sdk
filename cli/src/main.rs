use revised_sdk::{engine, report};
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
    let project = dir.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
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
