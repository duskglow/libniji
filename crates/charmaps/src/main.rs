//! `niji-charmaps DIR`: write every charmap of [`niji_charmaps::CHARSETS`] into `DIR/<name>`
//! (DIR is emptied first). `cargo xtask so` runs it and compresses the files for the `locales`
//! package.

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let (Some(dir), None) = (args.next(), args.next()) else {
        eprintln!("usage: niji-charmaps DIR");
        return ExitCode::from(2);
    };
    let dir = Path::new(&dir);
    let cleared = if dir.exists() { std::fs::remove_dir_all(dir) } else { Ok(()) };
    if let Err(e) = cleared.and_then(|()| std::fs::create_dir_all(dir)) {
        eprintln!("niji-charmaps: {}: {e}", dir.display());
        return ExitCode::FAILURE;
    }
    for (name, codec) in niji_charmaps::CHARSETS {
        if let Err(e) = std::fs::write(dir.join(name), niji_charmaps::charmap(name, codec)) {
            eprintln!("niji-charmaps: {}/{name}: {e}", dir.display());
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}
