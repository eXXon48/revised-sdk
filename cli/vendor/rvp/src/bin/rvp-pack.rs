//! rvp-pack — build `.rvp` packages. Prints BLAKE2b-512(core) (the catalog pin) on stdout.
//!
//!   rvp-pack --emit-core <src_dir> <out.core>
//!   rvp-pack --from-core <core> <out.rvp> [--sig <core.minisig>]
//!   rvp-pack <src_dir> <out.rvp> [--sig <core.minisig>]
//!   rvp-pack --extract-core <in.rvp> <out.core>   (signed or unsigned input)
//!
//! `--extra-stock <paths-file>` (any position, any form) appends `<f|d|l> <abs path>` lines to the
//! built-in stock list, so test packages can modify fixture files that are not in stock.
use std::path::Path;
use std::process::ExitCode;

use rvp::container::{container_read, container_write};
use rvp::hash::blake2b512_hex;
use rvp::pack::{check_signable, pack_dir, unpack_mem};
use rvp::smsvc::sm_services;
use rvp::stock::{check_stock, Stock};

const USAGE: &str = "usage:
  rvp-pack --emit-core <src_dir> <out.core>
  rvp-pack --from-core <core> <out.rvp> [--sig <core.minisig>]
  rvp-pack <src_dir> <out.rvp> [--sig <core.minisig>]
  rvp-pack --extract-core <in.rvp> <out.core>
any form also takes: [--extra-stock <paths-file>] (extra stock paths, for test fixtures)";

fn read(p: &str) -> Result<Vec<u8>, String> {
    std::fs::read(p).map_err(|e| format!("{p}: {e}"))
}

fn write(p: &str, b: &[u8]) -> Result<(), String> {
    std::fs::write(p, b).map_err(|e| format!("{p}: {e}"))
}

fn wrap(core: &[u8], out: &str, sig: Option<&str>) -> Result<(), String> {
    let sig = sig.map(read).transpose()?;
    write(
        out,
        &container_write(core, sig.as_deref()).map_err(|e| e.to_string())?,
    )
}

/// Stock 74.00.324A rootfs path list: overrides are checked against it before anything is written.
const STOCK: &str = include_str!("../../stock/74.00.324A.paths");

/// Validated core + its stock-override check (Package Format §5 Target-location policy).
fn checked(core: Vec<u8>, extra: &str) -> Result<Vec<u8>, String> {
    let m = check_signable(&core).map_err(|e| e.to_string())?;
    check_stock(&m, &Stock::parse(&format!("{STOCK}\n{extra}"))).map_err(|e| e.to_string())?;
    let (m, entries) = unpack_mem(&core).map_err(|e| e.to_string())?;
    sm_services(&m, &entries).map_err(|e| e.to_string())?;
    Ok(core)
}

fn pack(src: &str, extra: &str) -> Result<Vec<u8>, String> {
    checked(pack_dir(Path::new(src)).map_err(|e| e.to_string())?, extra)
}

fn run(args: &[String]) -> Result<(), String> {
    let mut a: Vec<&str> = args.iter().map(String::as_str).collect();
    let mut extra = String::new();
    if let Some(i) = a.iter().position(|x| *x == "--extra-stock") {
        let f = *a.get(i + 1).ok_or(USAGE)?;
        extra = String::from_utf8(read(f)?).map_err(|e| format!("{f}: {e}"))?;
        a.drain(i..i + 2);
    }
    let extra = extra.as_str();
    let core = match a.as_slice() {
        ["--emit-core", src, out] => {
            let core = pack(src, extra)?;
            write(out, &core)?;
            core
        }
        ["--extract-core", rvp, out] => {
            let core = container_read(&read(rvp)?).map_err(|e| e.to_string())?.core;
            write(out, &core)?;
            core
        }
        ["--from-core", core, out, rest @ ..] => {
            let core = checked(read(core)?, extra)?;
            wrap(&core, out, sig_arg(rest)?)?;
            core
        }
        [src, out, rest @ ..] if !src.starts_with("--") => {
            let core = pack(src, extra)?;
            wrap(&core, out, sig_arg(rest)?)?;
            core
        }
        _ => return Err(USAGE.into()),
    };
    println!("{}", blake2b512_hex(&core));
    Ok(())
}

fn sig_arg<'a>(rest: &[&'a str]) -> Result<Option<&'a str>, String> {
    match rest {
        [] => Ok(None),
        ["--sig", s] => Ok(Some(s)),
        _ => Err(USAGE.into()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("rvp-pack: {e}");
            ExitCode::FAILURE
        }
    }
}
