//! The runner: one side (reference or Liella) of one transcript in a private root of its own.
//!
//! 1. The private root is a new directory under `$TMPDIR`; the `dir:`, `file:`, `link:` and `cache:` items are
//!    created in it, and the tree is read (sorted, so no file system's directory order matters).
//! 2. The command runs with the transcript's argv (argv[0] is the bare command name on both sides), exactly the
//!    transcript's environment (nothing inherited), its stdin, and the private root as the working directory.
//!    `@ROOT@` in arguments, environment values, stdin and link targets stands for the private root's path.
//!    `namespace: user|root` runs it through `unshare --user --map-root-user --mount` and this binary's
//!    `transcript-exec` helper, which mounts the binds and then runs the command (as uid 0 in the namespace, or,
//!    for `user`, through a nested user namespace that maps the caller's own uid and gid back).
//! 3. The tree is read again; what changed is the `written` list.
//! 4. Normalisation, the only one by default: the private root's path becomes `@ROOT@` in both streams, in written
//!    files and in link targets. Each `normalise:` line of the transcript then replaces one more literal.

use super::format::{ElfKind, Namespace, Outcome, ROOT_TOKEN, Setup, Status, Transcript, Written};
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

/// The helper subcommand that runs inside the namespace (dispatched before the tree root is looked up).
pub(crate) const EXEC_HELPER: &str = "transcript-exec";
/// The helper's exit status when it fails itself; with [`HELPER_PREFIX`] on stderr it is a harness error, never an
/// outcome to compare.
const HELPER_FAILED: i32 = 125;
const HELPER_PREFIX: &[u8] = b"xtask transcript-exec: ";

/// One side of a comparison: its label and the program run as the transcript's command.
pub(crate) struct Side<'a> {
    pub label: &'a str,
    pub program: PathBuf,
}

/// A system tool by name, from the standard directories only (the transcript's environment has no PATH to trust).
pub(crate) fn tool(name: &str) -> Option<PathBuf> {
    ["/usr/bin", "/bin", "/usr/sbin", "/sbin"].iter().map(|d| Path::new(d).join(name)).find(|p| p.is_file())
}

/// Whether unprivileged user and mount namespaces work here, and why not when they do not.
pub(crate) fn namespaces() -> Result<(), String> {
    static WHY: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    WHY.get_or_init(|| {
        let unshare = tool("unshare").ok_or("no unshare(1) in /usr/bin or /bin")?;
        let o = Command::new(&unshare).args(["--user", "--map-root-user", "--mount", "--", "true"]).stdin(Stdio::null()).output().map_err(|e| format!("{}: {e}", unshare.display()))?;
        if o.status.success() { Ok(()) } else { Err(format!("`unshare --user --map-root-user --mount true` failed: {}", String::from_utf8_lossy(&o.stderr).trim())) }
    })
    .clone()
}

/// Replace every `from` in `hay` with `to`.
pub(crate) fn replace(hay: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    if from.is_empty() {
        return hay.to_vec();
    }
    let mut out = Vec::with_capacity(hay.len());
    let mut i = 0;
    while i < hay.len() {
        if hay[i..].starts_with(from) {
            out.extend_from_slice(to);
            i += from.len();
        } else {
            out.push(hay[i]);
            i += 1;
        }
    }
    out
}

/// A node of the private root's tree.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Node {
    File(Vec<u8>),
    Dir,
    Link(Vec<u8>),
    Other(u32),
}

/// The tree under `root`, by path relative to it (bytes), read in sorted order.
fn snapshot(root: &Path) -> Result<BTreeMap<Vec<u8>, Node>, String> {
    fn walk(dir: &Path, rel: &[u8], out: &mut BTreeMap<Vec<u8>, Node>) -> Result<(), String> {
        let mut names: Vec<OsString> = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?.map(|e| e.map(|e| e.file_name())).collect::<Result<_, _>>().map_err(|e| format!("{}: {e}", dir.display()))?;
        names.sort();
        for name in names {
            let path = dir.join(&name);
            let mut r = rel.to_vec();
            if !r.is_empty() {
                r.push(b'/');
            }
            r.extend_from_slice(name.as_bytes());
            let md = std::fs::symlink_metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let ft = md.file_type();
            let node = if ft.is_symlink() {
                Node::Link(std::fs::read_link(&path).map_err(|e| format!("{}: {e}", path.display()))?.into_os_string().into_vec())
            } else if ft.is_dir() {
                walk(&path, &r, out)?;
                Node::Dir
            } else if ft.is_file() {
                Node::File(std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?)
            } else {
                Node::Other(md.mode() & 0o170000)
            };
            out.insert(r, node);
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    walk(root, b"", &mut out)?;
    Ok(out)
}

/// What changed from `before` to `after`, sorted by path.
fn written(before: &BTreeMap<Vec<u8>, Node>, after: &BTreeMap<Vec<u8>, Node>) -> Vec<(Vec<u8>, Written)> {
    let mut out = BTreeMap::new();
    for (p, n) in after {
        if before.get(p) != Some(n) {
            let w = match n {
                Node::File(b) => Written::File(b.clone()),
                Node::Dir => Written::Dir,
                Node::Link(t) => Written::Link(t.clone()),
                Node::Other(m) => Written::Other(*m),
            };
            out.insert(p.clone(), w);
        }
    }
    for p in before.keys() {
        if !after.contains_key(p) {
            out.insert(p.clone(), Written::Removed);
        }
    }
    out.into_iter().collect()
}

/// Create the transcript's items under `root`.
fn setup(t: &Transcript, root: &Path) -> Result<(), String> {
    let rootb = root.as_os_str().as_bytes();
    let at = |p: &[u8]| -> Result<PathBuf, String> {
        let path = root.join(OsStr::from_bytes(p));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        Ok(path)
    };
    for item in &t.setup {
        match item {
            Setup::Dir(p) => {
                let path = at(p)?;
                std::fs::create_dir_all(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            }
            Setup::File(p, body) => {
                let path = at(p)?;
                std::fs::write(&path, body).map_err(|e| format!("{}: {e}", path.display()))?;
            }
            Setup::Link(p, target) => {
                let path = at(p)?;
                std::os::unix::fs::symlink(OsStr::from_bytes(&replace(target, ROOT_TOKEN, rootb)), &path).map_err(|e| format!("{}: {e}", path.display()))?;
            }
            Setup::Cache(p, generator, entries) => {
                use liella_ld::cache;
                let mut rows: Vec<cache::Entry<'_>> = entries
                    .iter()
                    .enumerate()
                    .map(|(i, e)| cache::Entry { flags: e.flags, key: &e.key, path: &e.path, hwcaps: e.hwcaps.as_deref(), order: i as u32 })
                    .collect();
                let what = |e| format!("cache {}: the writer refused it: {e:?}", String::from_utf8_lossy(p));
                let len = cache::encoded_len(&rows, generator).map_err(what)?;
                let mut buf = vec![0u8; len];
                let n = cache::write(&mut rows, generator, &mut buf).map_err(what)?;
                buf.truncate(n);
                let path = at(p)?;
                std::fs::write(&path, &buf).map_err(|e| format!("{}: {e}", path.display()))?;
            }
            Setup::Elf(p, soname, cut, kind) => {
                let mut image = match kind {
                    ElfKind::X86_64 => elf_image(soname.as_deref()),
                    ElfKind::I386 => elf32_image(soname.as_deref(), 3),
                    ElfKind::X32 => elf32_image(soname.as_deref(), 62),
                };
                if let Some(n) = cut {
                    image.truncate(*n);
                }
                let path = at(p)?;
                std::fs::write(&path, &image).map_err(|e| format!("{}: {e}", path.display()))?;
            }
        }
    }
    Ok(())
}

/// A minimal x86-64 shared object: the ELF header, PT_LOAD over the whole file and PT_DYNAMIC, a dynamic section
/// with DT_STRTAB, DT_STRSZ and (with `soname`) DT_SONAME, then the string table (ELF-64 Object File Format,
/// System V psABI x86-64). Virtual addresses equal file offsets.
pub(crate) fn elf_image(soname: Option<&[u8]>) -> Vec<u8> {
    const PH: usize = 64;
    const DYN: usize = PH + 2 * 56;
    let ndyn = if soname.is_some() { 4 } else { 3 };
    let strtab = DYN + ndyn * 16;
    let mut strings = vec![0u8];
    if let Some(s) = soname {
        strings.extend_from_slice(s);
        strings.push(0);
    }
    let total = (strtab + strings.len()) as u64;
    let mut b = Vec::with_capacity(total as usize);
    b.extend_from_slice(b"\x7fELF\x02\x01\x01\x00");
    b.resize(16, 0);
    let h16 = |b: &mut Vec<u8>, v: u16| b.extend_from_slice(&v.to_le_bytes());
    let h32 = |b: &mut Vec<u8>, v: u32| b.extend_from_slice(&v.to_le_bytes());
    let h64 = |b: &mut Vec<u8>, v: u64| b.extend_from_slice(&v.to_le_bytes());
    h16(&mut b, 3); // ET_DYN
    h16(&mut b, 62); // EM_X86_64
    h32(&mut b, 1);
    h64(&mut b, 0); // e_entry
    h64(&mut b, PH as u64);
    h64(&mut b, 0); // e_shoff
    h32(&mut b, 0);
    for v in [64u16, 56, 2, 64, 0, 0] {
        h16(&mut b, v);
    }
    // PT_LOAD, R, the whole file.
    for (ty, flags, off, size, align) in [(1u32, 4u32, 0u64, total, 0x1000u64), (2, 6, DYN as u64, (ndyn * 16) as u64, 8)] {
        h32(&mut b, ty);
        h32(&mut b, flags);
        h64(&mut b, off);
        h64(&mut b, off);
        h64(&mut b, off);
        h64(&mut b, size);
        h64(&mut b, size);
        h64(&mut b, align);
    }
    if soname.is_some() {
        h64(&mut b, 14); // DT_SONAME
        h64(&mut b, 1);
    }
    for (tag, val) in [(5u64, strtab as u64), (10, strings.len() as u64), (0, 0)] {
        h64(&mut b, tag);
        h64(&mut b, val);
    }
    b.extend_from_slice(&strings);
    b
}

/// A minimal ELF32 little-endian shared object of machine `machine` (3 = EM_386 for i386, 62 = EM_X86_64 for x32):
/// the ELF-32 header, PT_LOAD over the whole file and PT_DYNAMIC, a dynamic section with DT_STRTAB, DT_STRSZ and
/// (with `soname`) DT_SONAME, then the string table (System V gABI, ELF-32 layout: a 52-byte header, 32-byte program
/// headers with `p_flags` after `p_memsz`, 8-byte dynamic entries). Virtual addresses equal file offsets.
pub(crate) fn elf32_image(soname: Option<&[u8]>, machine: u16) -> Vec<u8> {
    const PH: usize = 52;
    const DYN: usize = PH + 2 * 32;
    let ndyn = if soname.is_some() { 4 } else { 3 };
    let strtab = DYN + ndyn * 8;
    let mut strings = vec![0u8];
    if let Some(s) = soname {
        strings.extend_from_slice(s);
        strings.push(0);
    }
    let total = (strtab + strings.len()) as u32;
    let mut b = Vec::with_capacity(total as usize);
    b.extend_from_slice(b"\x7fELF\x01\x01\x01\x00");
    b.resize(16, 0);
    let h16 = |b: &mut Vec<u8>, v: u16| b.extend_from_slice(&v.to_le_bytes());
    let h32 = |b: &mut Vec<u8>, v: u32| b.extend_from_slice(&v.to_le_bytes());
    h16(&mut b, 3); // ET_DYN
    h16(&mut b, machine);
    h32(&mut b, 1);
    h32(&mut b, 0); // e_entry
    h32(&mut b, PH as u32);
    h32(&mut b, 0); // e_shoff
    h32(&mut b, 0);
    for v in [52u16, 32, 2, 40, 0, 0] {
        h16(&mut b, v);
    }
    for (ty, flags, off, size, align) in [(1u32, 4u32, 0u32, total, 0x1000u32), (2, 6, DYN as u32, (ndyn * 8) as u32, 4)] {
        for v in [ty, off, off, off, size, size, flags, align] {
            h32(&mut b, v);
        }
    }
    if soname.is_some() {
        h32(&mut b, 14); // DT_SONAME
        h32(&mut b, 1);
    }
    for (tag, val) in [(5u32, strtab as u32), (10, strings.len() as u32), (0, 0)] {
        h32(&mut b, tag);
        h32(&mut b, val);
    }
    b.extend_from_slice(&strings);
    b
}

/// A written loader cache as text: one `0x<flags> <key> <path>[ hwcaps=<name>]` line per entry, in cache order,
/// without the generator text (an identity string, Q3 of plan/utilities.md) or anything else of the layout.
pub(crate) fn cache_text(bytes: &[u8]) -> Vec<u8> {
    let Some(c) = liella_ld::cache::Cache::parse(bytes) else { return format!("not a loader cache ({} bytes)\n", bytes.len()).into_bytes() };
    let mut out = Vec::new();
    for i in 0..c.len() {
        let Some(e) = c.get(i) else {
            out.extend_from_slice(format!("entry {i} unreadable\n").as_bytes());
            continue;
        };
        out.extend_from_slice(format!("0x{:x} ", e.flags).as_bytes());
        out.extend_from_slice(e.key);
        out.push(b' ');
        out.extend_from_slice(e.path);
        if let Some(h) = e.hwcaps {
            out.extend_from_slice(b" hwcaps=");
            out.extend_from_slice(h);
        }
        out.push(b'\n');
    }
    out
}

/// The process's own uid and gid (the owner of `/proc/self`), for the nested namespace's map.
fn ids() -> Result<(u32, u32), String> {
    let md = std::fs::metadata("/proc/self").map_err(|e| format!("/proc/self: {e}"))?;
    Ok((md.uid(), md.gid()))
}

/// Run one side of `t` in a new private root under `scratch`, and return its normalised outcome. An `Err` is a
/// harness failure (setup, spawn, namespace helper), never a difference to compare.
pub(crate) fn run_side(t: &Transcript, side: &Side<'_>, scratch: &Path) -> Result<Outcome, String> {
    let root = scratch.join(side.label);
    std::fs::create_dir(&root).map_err(|e| format!("{}: {e}", root.display()))?;
    let root = root.canonicalize().map_err(|e| format!("{}: {e}", root.display()))?;
    let result = run_in(t, side, &root);
    // The command may leave read-only directories: make them writable so the root can be removed.
    if let Ok(tree) = snapshot(&root) {
        for (p, _) in tree.iter().filter(|(_, n)| **n == Node::Dir) {
            let _ =std::fs::set_permissions(root.join(OsStr::from_bytes(p)), std::fs::Permissions::from_mode(0o700));
        }
    }
    let _ = std::fs::remove_dir_all(&root);
    result
}

fn run_in(t: &Transcript, side: &Side<'_>, root: &Path) -> Result<Outcome, String> {
    setup(t, root)?;
    let before = snapshot(root)?;
    let rootb = root.as_os_str().as_bytes();
    let sub = |b: &[u8]| OsString::from_vec(replace(b, ROOT_TOKEN, rootb));
    let args: Vec<OsString> = t.args.iter().map(|a| sub(a)).collect();
    let mut cmd = if t.namespace == Namespace::None {
        let mut c = Command::new(&side.program);
        c.arg0(&t.command).args(&args);
        c
    } else {
        namespaces()?;
        let unshare = tool("unshare").ok_or("no unshare(1)")?;
        let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
        let mut c = Command::new(unshare);
        c.args(["--user", "--map-root-user", "--mount", "--"]).arg(exe).arg(EXEC_HELPER);
        for (src, dst) in &t.binds {
            c.arg("--bind").arg(root.join(OsStr::from_bytes(src))).arg(OsStr::from_bytes(dst));
        }
        if t.namespace == Namespace::User {
            let (uid, gid) = ids()?;
            c.arg("--drop").arg(uid.to_string()).arg(gid.to_string());
        }
        c.arg("--").arg(&side.program).arg(&t.command).args(&args);
        c
    };
    cmd.env_clear();
    for (k, v) in &t.env {
        cmd.env(OsStr::from_bytes(k), sub(v));
    }
    cmd.current_dir(root).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("{} {}: {e}", side.label, side.program.display()))?;
    let mut stdin = child.stdin.take().ok_or("no stdin pipe")?;
    let data = replace(&t.stdin, ROOT_TOKEN, rootb);
    // The command may exit without reading its stdin: a broken pipe is not an error.
    let feeder = std::thread::spawn(move || {
        let _ = stdin.write_all(&data);
    });
    let out = child.wait_with_output().map_err(|e| format!("{}: {e}", side.label))?;
    let _ = feeder.join();
    let status = match (out.status.code(), out.status.signal()) {
        (Some(c), _) => Status::Exit(c),
        (None, Some(s)) => Status::Signal(s),
        (None, None) => return Err(format!("{}: no exit status", side.label)),
    };
    if status == Status::Exit(HELPER_FAILED) && out.stderr.starts_with(HELPER_PREFIX) {
        return Err(format!("{}: {}", side.label, String::from_utf8_lossy(&out.stderr).trim()));
    }
    let after = snapshot(root)?;
    let mut o = Outcome { status, stdout: out.stdout, stderr: out.stderr, written: written(&before, &after) };
    for (p, w) in &mut o.written {
        if let (true, Written::File(b)) = (t.as_cache.contains(p), &*w) {
            *w = Written::File(cache_text(b));
        }
    }
    normalise(&mut o, rootb, &t.normalise);
    Ok(o)
}

/// The private root's path, then each extra rule, in streams, written files and link targets.
pub(crate) fn normalise(o: &mut Outcome, root: &[u8], extra: &[(Vec<u8>, Vec<u8>)]) {
    let rules: Vec<(&[u8], &[u8])> = std::iter::once((root, ROOT_TOKEN)).chain(extra.iter().map(|(a, b)| (a.as_slice(), b.as_slice()))).collect();
    let apply = |b: &mut Vec<u8>| {
        for (from, to) in &rules {
            *b = replace(b, from, to);
        }
    };
    apply(&mut o.stdout);
    apply(&mut o.stderr);
    for (_, w) in &mut o.written {
        match w {
            Written::File(b) | Written::Link(b) => apply(b),
            _ => {}
        }
    }
}

/// Replace the rest of every stdout and stderr line that starts with one of `prefixes` by `token`
/// (`host-value:` and `mask-value:` lines of a transcript).
pub(crate) fn mask_lines(o: &mut Outcome, prefixes: &[Vec<u8>], token: &[u8]) {
    if prefixes.is_empty() {
        return;
    }
    let apply = |b: &mut Vec<u8>| {
        let mut out = Vec::with_capacity(b.len());
        for line in b.split_inclusive(|&c| c == b'\n') {
            match prefixes.iter().find(|p| line.starts_with(p)) {
                Some(p) => {
                    out.extend_from_slice(p);
                    out.extend_from_slice(token);
                    if line.ends_with(b"\n") {
                        out.push(b'\n');
                    }
                }
                None => out.extend_from_slice(line),
            }
        }
        *b = out;
    };
    apply(&mut o.stdout);
    apply(&mut o.stderr);
}

/// `xtask transcript-exec [--bind SRC DST]...[--drop UID GID] -- PROGRAM ARGV0 ARGS...`, run by `unshare` inside the
/// new user and mount namespace: mount the binds, then run the command (through a nested user namespace mapping
/// UID and GID back, with `--drop`). It replaces itself with the command, so the outcome is the command's own.
pub(crate) fn exec_helper(args: &[OsString]) -> ExitCode {
    let fail = |m: String| {
        let mut e = std::io::stderr();
        let _ = e.write_all(HELPER_PREFIX);
        let _ = writeln!(e, "{m}");
        ExitCode::from(HELPER_FAILED as u8)
    };
    let mut binds = Vec::new();
    let mut drop = None;
    let mut i = 0;
    while i < args.len() && args[i] != "--" {
        match (args[i].to_str(), args.get(i + 1), args.get(i + 2)) {
            (Some("--bind"), Some(s), Some(d)) => binds.push((s.clone(), d.clone())),
            (Some("--drop"), Some(u), Some(g)) => drop = Some((u.clone(), g.clone())),
            _ => return fail(format!("bad arguments {args:?}")),
        }
        i += 3;
    }
    let rest = args.get(i + 1..).unwrap_or_default();
    if rest.len() < 2 {
        return fail(format!("no command in {args:?}"));
    }
    let Some(mount) = tool("mount") else { return fail("no mount(8)".into()) };
    for (src, dst) in &binds {
        match Command::new(&mount).arg("--bind").arg(src).arg(dst).output() {
            Ok(o) if o.status.success() => {}
            Ok(o) => return fail(format!("mount --bind {} {}: {}", Path::new(src).display(), Path::new(dst).display(), String::from_utf8_lossy(&o.stderr).trim())),
            Err(e) => return fail(format!("{}: {e}", mount.display())),
        }
    }
    let err = if let Some((uid, gid)) = drop {
        let (Some(unshare), Ok(exe)) = (tool("unshare"), std::env::current_exe()) else { return fail("no unshare(1) or current_exe".into()) };
        let mut m_user = OsString::from("--map-user=");
        m_user.push(&uid);
        let mut m_group = OsString::from("--map-group=");
        m_group.push(&gid);
        Command::new(unshare).arg("--user").arg(m_user).arg(m_group).arg("--").arg(exe).arg(EXEC_HELPER).arg("--").args(rest).exec()
    } else {
        Command::new(&rest[0]).arg0(&rest[1]).args(&rest[2..]).exec()
    };
    fail(format!("exec {}: {err}", Path::new(&rest[0]).display()))
}

#[cfg(test)]
mod tests {
    use super::super::format::parse;
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("xtask-transcript-test-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_side_sees_only_its_argv_env_stdin_and_root_and_its_writes_are_recorded() {
        // Trace: no spec clause (test-harness tooling; UTIL-HARNESS runner).
        let t = parse(
            "name: n\nclause: none (runner test)\ncommand: probe\nenv: X=@ROOT@/x\narg: -c\n\
             arg: printf '%s|%s|%s|' \"$0\" \"$X\" \"$(pwd)\"; cat; echo e >&2; printf w > out; mkdir d; ln -s @ROOT@/out d/l; rm gone; printf y > kept; exit 3\n\
             stdin:\n| in\nfile: gone\n| x\nfile: kept\n| x\nfile: same\n| x\n",
            "t",
        )
        .unwrap();
        let s = scratch("side");
        let o = run_side(&t, &Side { label: "a", program: "/bin/sh".into() }, &s).unwrap();
        assert_eq!(o.status, Status::Exit(3));
        assert_eq!(String::from_utf8_lossy(&o.stdout), "probe|@ROOT@/x|@ROOT@|in\n");
        assert_eq!(o.stderr, b"e\n");
        let w: Vec<(String, Written)> = o.written.into_iter().map(|(p, w)| (String::from_utf8(p).unwrap(), w)).collect();
        assert_eq!(
            w,
            vec![
                ("d".into(), Written::Dir),
                ("d/l".into(), Written::Link(b"@ROOT@/out".to_vec())),
                ("gone".into(), Written::Removed),
                ("kept".into(), Written::File(b"y".to_vec())),
                ("out".into(), Written::File(b"w".to_vec())),
            ]
        );
        assert_eq!(std::fs::read_dir(&s).unwrap().count(), 0, "the private root is removed");
        std::fs::remove_dir_all(&s).unwrap();
    }

    #[test]
    fn normalise_replaces_the_root_then_the_listed_literals_only() {
        // Trace: no spec clause (test-harness tooling; UTIL-HARNESS normalisation).
        let mut o = Outcome { status: Status::Exit(0), stdout: b"/r/x /sbin/a.real: /r\n".to_vec(), stderr: b"/r".to_vec(), written: vec![(b"l".to_vec(), Written::Link(b"/r/t".to_vec()))] };
        normalise(&mut o, b"/r", &[(b"/sbin/a.real".to_vec(), b"a".to_vec())]);
        assert_eq!(o.stdout, b"@ROOT@/x a: @ROOT@\n");
        assert_eq!(o.stderr, b"@ROOT@");
        assert_eq!(o.written[0].1, Written::Link(b"@ROOT@/t".to_vec()));
        let mut p = o.clone();
        normalise(&mut p, b"/elsewhere", &[]);
        assert_eq!(p, o, "nothing else is normalised by default");
    }
}
