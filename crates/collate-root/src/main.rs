//! `niji-collate-root REPO OUT`: write the root `LC_COLLATE` locale source ([`niji_collate_root::source`])
//! built from the spec files under the repository root `REPO` into the file `OUT`.

#![forbid(unsafe_code)]

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let (Some(repo), Some(out), None) = (args.next(), args.next(), args.next()) else {
        eprintln!("usage: niji-collate-root REPO OUT");
        return ExitCode::from(2);
    };
    match niji_collate_root::source(Path::new(&repo)).and_then(|s| std::fs::write(&out, s).map_err(|e| format!("{}: {e}", Path::new(&out).display()))) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("niji-collate-root: {e}");
            ExitCode::FAILURE
        }
    }
}
