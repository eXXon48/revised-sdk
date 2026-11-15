//! rvp-verify <file.rvp> <pubkey_file> — exit 0 iff the embedded signature is valid.
use std::process::ExitCode;

use rvp::verify::{verify_container, PubKey};

fn run(args: &[String]) -> Result<(), String> {
    let [rvp, pubkey] = args else {
        return Err("usage: rvp-verify <file.rvp> <pubkey_file>".into());
    };
    let rvp = std::fs::read(rvp).map_err(|e| format!("{rvp}: {e}"))?;
    let pk = std::fs::read_to_string(pubkey).map_err(|e| format!("{pubkey}: {e}"))?;
    let pk = PubKey::parse(&pk).map_err(|e| e.to_string())?;
    verify_container(&rvp, &pk).map_err(|e| e.to_string())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => {
            println!("ok");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("rvp-verify: {e}");
            ExitCode::FAILURE
        }
    }
}
