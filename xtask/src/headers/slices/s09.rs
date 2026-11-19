//! Slice 9 (plan/headers.md section 4: Linux process and event interfaces): the marker of `sys/syscall.h`.
//! Listed by `xtask/build.rs` (plan/headers.md section 4, "Adding a slice").
//!
//! * `@@syscalls`: `SYS_<name>` for each `__NR_<name>` of `spec/abi/syscalls.tsv` (the kernel UAPI's names), each
//!   under `#ifdef __NR_<name>` so a kernel header without it still compiles.
use super::super::{tsv, Gen, Marker, Slice};

pub(super) const SLICE: Slice = Slice { markers: &[Marker { name: "@@syscalls", run: syscalls }], inline: &[], linked_elsewhere: &[] };

fn syscalls(g: &mut Gen<'_>, _args: &[&str]) {
    match tsv(g.root, "spec/abi/syscalls.tsv") {
        Ok(rows) => {
            for n in rows.iter().map(|r| &r["name"]) {
                g.out.extend([format!("#ifdef __NR_{n}"), format!("# ifndef SYS_{n}"), format!("#  define SYS_{n} __NR_{n}"), "# endif".into(), "#endif".into()]);
            }
        }
        Err(e) => g.errors.push(format!("{}: @@syscalls: {e}", g.header)),
    }
}
