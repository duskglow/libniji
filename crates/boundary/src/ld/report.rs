//! `ld.so --list-tunables` and `ld.so --list-diagnostics` (ld.so(8); LDSO-PROGRAM-OPTIONS-REST-2,
//! D-27.88): the reference's line forms (black-box probes), libniji's own content
//! (docs/differences-from-glibc.md). Program mode only: cold, and panic-free (each panic site of the
//! loader is a relocation every start pays for).

use niji::config::Tunables;
use niji_ld::arch::{Arch, Native};
use niji_ld::program::{self, number};
use niji_ld::search;
use super::io::{exit, write_all};

/// Standard output, written a buffer at a time.
struct Out {
    buf: [u8; 512],
    n: usize,
}

impl Out {
    fn new() -> Out {
        Out { buf: [0; 512], n: 0 }
    }

    fn put(&mut self, s: &[u8]) {
        for &b in s {
            if self.n >= self.buf.len() {
                self.flush();
            }
            if let Some(slot) = self.buf.get_mut(self.n) {
                *slot = b;
                self.n = self.n.wrapping_add(1);
            }
        }
    }

    fn flush(&mut self) {
        write_all(1, self.buf.get(..self.n).unwrap_or_default());
        self.n = 0;
    }

    /// `n` in hexadecimal with `0x` (or decimal).
    fn num(&mut self, n: u64, hex: bool) {
        let mut b = [0u8; 24];
        self.put(number(n, hex, &mut b));
    }

    /// `s` quoted, then a newline ([`program::quoted_byte`]).
    fn quoted(&mut self, s: &[u8]) {
        self.put(b"\"");
        for &b in s {
            let mut q = [0u8; 4];
            self.put(program::quoted_byte(b, &mut q));
        }
        self.put(b"\"\n");
    }

    /// `key[0xI]` (`key[0xI].` + `field` when `field` is not empty).
    fn indexed(&mut self, key: &[u8], i: usize, field: &[u8]) {
        self.put(key);
        self.put(b"[");
        self.num(u64::try_from(i).unwrap_or(u64::MAX), true);
        self.put(b"]");
        self.put(field);
    }
}

/// One `name: value (min: x, max: y)` line: a size in hexadecimal, a count in decimal.
fn tunable(o: &mut Out, name: &[u8], value: u64, (min, max): (u64, u64), hex: bool) {
    o.put(name);
    o.put(b": ");
    o.num(value, hex);
    o.put(b" (min: ");
    o.num(min, hex);
    o.put(b", max: ");
    o.num(max, hex);
    o.put(b")\n");
}

/// `ld.so --list-tunables`: libniji's tunables (docs/tunables.md) in the reference's form, each with
/// its value from `GLIBC_TUNABLES` (`text`; none honoured when `secure`) or its default; a threshold
/// left to the built-in choice reads 0. Exits 0.
#[cold]
#[inline(never)]
pub(super) fn tunables(text: Option<&[u8]>, secure: bool) -> ! {
    let t = text.map(|t| Tunables::parse(t, secure)).unwrap_or_default();
    let perturb = text.and_then(|t| Tunables::malloc_perturb(t, secure)).unwrap_or(0);
    let flag = |v: Option<bool>, default: bool| u64::from(v.unwrap_or(default));
    let size = |v: Option<usize>| v.and_then(|v| u64::try_from(v).ok()).unwrap_or(0);
    let mut o = Out::new();
    // The value given, or its default (D-27.94: the surplus is the value plus a fixed 1200 bytes).
    let optional = t.optional_static_tls.unwrap_or(niji_ld::tls::OPTIONAL_STATIC_TLS);
    tunable(&mut o, b"glibc.rtld.optional_static_tls", optional, (0, u64::MAX), true);
    tunable(&mut o, b"niji.compat.execstack", flag(t.exec_stack, false), (0, 1), false);
    tunable(&mut o, b"niji.compat.no_classed_swap", flag(t.no_classed_swap, true), (0, 1), false);
    // A string tunable: the choices made, in the tunable's own syntax (`-` disables).
    o.put(b"niji.cpu.hwcaps:");
    let mut sep: &[u8] = b" ";
    for (name, v) in [(&b"AVX2"[..], t.avx2), (b"ERMS", t.erms), (b"FMA", t.fma)] {
        if let Some(on) = v {
            o.put(sep);
            o.put(if on { b"" } else { b"-" });
            o.put(name);
            sep = b",";
        }
    }
    o.put(b"\n");
    tunable(&mut o, b"niji.cpu.rep_movsb_threshold", size(t.rep_movsb_threshold), (0x100, u64::MAX), true);
    tunable(&mut o, b"niji.cpu.rep_stosb_threshold", size(t.rep_stosb_threshold), (1, u64::MAX), true);
    tunable(&mut o, b"niji.malloc.perturb", u64::from(perturb), (0, 255), false);
    tunable(&mut o, b"niji.pthread.rseq", flag(t.rseq, true), (0, 1), false);
    o.flush();
    exit(0)
}

/// Whether `--list-diagnostics` shows the value of the environment entry named `name` (libniji's
/// list: the loader's and the C library's own settings); any other entry is listed by name only.
fn shown(name: &[u8]) -> bool {
    [&b"LD_"[..], b"MALLOC_", b"LC_"].iter().any(|p| name.starts_with(p))
        || [&b"LANG"[..], b"LANGUAGE", b"GLIBC_TUNABLES", b"LOCPATH", b"GCONV_PATH", b"TZ", b"TZDIR", b"NLSPATH"].contains(&name)
}

/// `ld.so --list-diagnostics`: `key=value` lines (strings quoted, numbers in hexadecimal) with
/// libniji's content: the `glibc-hwcaps` subdirectories searched (`hwcaps`), the page size, the
/// platform, the loader's and the C library's names, the environment (`envp`), the default
/// directories, the versions and the auxiliary vector (`auxv`, its words up to `AT_NULL`). Exits 0.
#[cold]
#[inline(never)]
pub(super) fn diagnostics(hwcaps: &[&[u8]], page: u64, platform: &str, envp: usize, auxv: &[u64]) -> ! {
    let mut o = Out::new();
    o.put(b"dl_hwcaps_subdirs=\"");
    for (i, h) in hwcaps.iter().enumerate() {
        o.put(if i == 0 { b"" } else { b":" });
        o.put(h);
    }
    o.put(b"\"\ndl_pagesize=");
    o.num(page, true);
    o.put(b"\ndl_platform=");
    o.quoted(platform.as_bytes());
    let interp = Native::INTERP.as_bytes();
    let base = interp.rsplit(|&b| b == b'/').next().unwrap_or(interp);
    o.put(b"dso.ld=");
    o.quoted(base);
    o.put(b"dso.libc=");
    o.quoted(b"libc.so.6");
    for (i, e) in super::entry::env_entries(envp).enumerate() {
        let name = e.split(|&b| b == b'=').next().unwrap_or(e);
        if shown(name) {
            o.indexed(b"env", i, b"=");
            o.quoted(e);
        } else {
            o.indexed(b"env_filtered", i, b"=");
            o.quoted(name);
        }
    }
    o.put(b"path.rtld=");
    o.quoted(interp);
    let mut i = 0usize;
    search::trusted_dirs::<Native>(&mut |d| {
        o.indexed(b"path.system_dirs", i, b"=\"");
        o.put(d);
        o.put(b"/\"\n");
        i = i.wrapping_add(1);
        true
    });
    o.put(b"version.libniji=");
    o.quoted(env!("CARGO_PKG_VERSION").as_bytes());
    o.put(b"version.version=\"2.39\"\n");
    for (i, pair) in auxv.chunks_exact(2).take_while(|p| p.first().is_some_and(|&t| t != 0)).enumerate() {
        let (t, v) = (pair.first().copied().unwrap_or(0), pair.get(1).copied().unwrap_or(0));
        o.indexed(b"auxv", i, b".a_type=");
        o.num(t, true);
        o.put(b"\n");
        o.indexed(b"auxv", i, b".a_val=");
        o.num(v, true);
        o.put(b"\n");
    }
    o.flush();
    exit(0)
}
