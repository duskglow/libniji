//! `ldconfig`'s engine (spec §31.2; plan/utilities.md UTIL-3) over an in-memory file system: the configuration
//! reader, the link rule, determinism and a seeded fuzz loop over configuration files. The transcripts
//! (validation/transcripts/ldconfig/) compare the same engine with the reference; these tests reach what a
//! transcript cannot: include cycles (a deliberate difference), listing order, and many random inputs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::as_conversions, missing_docs)]

use niji_ld::arch::X86_64;
use niji_ld::ldconfig::{run, Fs, Kind, Meta, Outcome, Stage, Text};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Node {
    File(Vec<u8>),
    Dir,
    Link(Vec<u8>),
}

/// A file system in a map from absolute path to node; `reverse` lists directories backwards.
#[derive(Clone, Default)]
struct Mem {
    nodes: BTreeMap<Vec<u8>, Node>,
    reverse: bool,
}

const ENOENT: i32 = 2;
const ENOTDIR: i32 = 20;
const ENOMEM: i32 = 12;
const ELOOP: i32 = 40;

fn comps(p: &[u8]) -> Vec<Vec<u8>> {
    p.split(|&c| c == b'/').filter(|c| !c.is_empty() && *c != b".").map(<[u8]>::to_vec).collect()
}

fn path_of(c: &[Vec<u8>]) -> Vec<u8> {
    let mut p = Vec::new();
    for x in c {
        p.push(b'/');
        p.extend_from_slice(x);
    }
    if p.is_empty() {
        p.push(b'/');
    }
    p
}

impl Mem {
    fn new() -> Self {
        let mut m = Mem::default();
        m.nodes.insert(b"/".to_vec(), Node::Dir);
        m
    }
    fn parents(&mut self, p: &[u8]) {
        let c = comps(p);
        for i in 0..c.len() {
            self.nodes.entry(path_of(&c[..i])).or_insert(Node::Dir);
        }
    }
    fn dir(&mut self, p: &str) -> &mut Self {
        self.parents(p.as_bytes());
        self.nodes.insert(path_of(&comps(p.as_bytes())), Node::Dir);
        self
    }
    fn file(&mut self, p: &str, body: &[u8]) -> &mut Self {
        self.parents(p.as_bytes());
        self.nodes.insert(p.as_bytes().to_vec(), Node::File(body.to_vec()));
        self
    }
    fn elf(&mut self, p: &str, soname: Option<&str>) -> &mut Self {
        let image = elf_image(soname.map(str::as_bytes));
        self.file(p, &image)
    }
    fn link(&mut self, p: &str, target: &str) -> &mut Self {
        self.parents(p.as_bytes());
        self.nodes.insert(p.as_bytes().to_vec(), Node::Link(target.as_bytes().to_vec()));
        self
    }
    /// The canonical path of `p`, symlinks resolved (the last one too when `follow`).
    fn resolve(&self, p: &[u8], follow: bool) -> Result<Vec<u8>, i32> {
        let mut todo: Vec<Vec<u8>> = comps(p).into_iter().rev().collect();
        let mut done: Vec<Vec<u8>> = Vec::new();
        let mut links = 0;
        while let Some(c) = todo.pop() {
            if c == b".." {
                done.pop();
                continue;
            }
            done.push(c);
            let here = path_of(&done);
            match self.nodes.get(&here) {
                Some(Node::Link(t)) if follow || !todo.is_empty() => {
                    links += 1;
                    if links > 40 {
                        return Err(ELOOP);
                    }
                    done.pop();
                    if t.first() == Some(&b'/') {
                        done.clear();
                    }
                    todo.extend(comps(t).into_iter().rev());
                }
                Some(Node::File(_)) if !todo.is_empty() => return Err(ENOTDIR),
                None if !todo.is_empty() => return Err(ENOENT),
                _ => {}
            }
        }
        Ok(path_of(&done))
    }
    fn meta(&self, p: &[u8], follow: bool) -> Result<Meta, i32> {
        let r = self.resolve(p, follow)?;
        let kind = match self.nodes.get(&r).ok_or(ENOENT)? {
            Node::File(_) => Kind::File,
            Node::Dir => Kind::Dir,
            Node::Link(_) => Kind::Link,
        };
        let ino = r.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3));
        Ok(Meta { kind, dev: 1, ino })
    }
    fn body(&self, p: &[u8]) -> Result<&[u8], i32> {
        match self.nodes.get(&self.resolve(p, true)?) {
            Some(Node::File(b)) => Ok(b),
            Some(_) => Err(21),
            None => Err(ENOENT),
        }
    }
    fn get(&self, p: &str) -> Option<&Node> {
        self.nodes.get(p.as_bytes())
    }
}

impl Fs for Mem {
    fn root(&self) -> &[u8] {
        b""
    }
    fn stat(&mut self, path: &[u8]) -> Result<Meta, i32> {
        self.meta(path, true)
    }
    fn lstat(&mut self, path: &[u8]) -> Result<Meta, i32> {
        self.meta(path, false)
    }
    fn list(&mut self, dir: &[u8], out: &mut Vec<Vec<u8>>) -> Result<(), i32> {
        let d = self.resolve(dir, true)?;
        if self.nodes.get(&d) != Some(&Node::Dir) {
            return Err(ENOTDIR);
        }
        let prefix = if d == b"/" { d.clone() } else { [d.as_slice(), b"/"].concat() };
        let mut names: Vec<Vec<u8>> = self
            .nodes
            .keys()
            .filter_map(|k| k.strip_prefix(prefix.as_slice()).filter(|r| !r.is_empty() && !r.contains(&b'/')).map(<[u8]>::to_vec))
            .collect();
        if self.reverse {
            names.reverse();
        }
        out.extend(names);
        Ok(())
    }
    fn read_at(&mut self, path: &[u8], offset: u64, buf: &mut [u8]) -> Result<usize, i32> {
        let b = self.body(path)?;
        let start = usize::try_from(offset).unwrap_or(usize::MAX).min(b.len());
        let n = buf.len().min(b.len() - start);
        buf[..n].copy_from_slice(&b[start..start + n]);
        Ok(n)
    }
    fn read_file(&mut self, path: &[u8], max: usize, out: &mut Vec<u8>) -> Result<(), i32> {
        if path.ends_with(b"/enomem.conf") {
            return Err(ENOMEM); // a file system that runs out of memory reading this file
        }
        let b = self.body(path)?;
        out.extend_from_slice(&b[..b.len().min(max)]);
        Ok(())
    }
    fn read_link(&mut self, path: &[u8], out: &mut Vec<u8>) -> Result<(), i32> {
        match self.nodes.get(&self.resolve(path, false)?) {
            Some(Node::Link(t)) => {
                out.extend_from_slice(t);
                Ok(())
            }
            _ => Err(22),
        }
    }
    fn replace_link(&mut self, target: &[u8], link: &[u8]) -> Result<(), i32> {
        let p = self.resolve(link, false)?;
        self.nodes.insert(p, Node::Link(target.to_vec()));
        Ok(())
    }
    fn remove(&mut self, path: &[u8]) -> Result<(), i32> {
        let p = self.resolve(path, false)?;
        self.nodes.remove(&p).map(|_| ()).ok_or(ENOENT)
    }
    fn replace_file(&mut self, path: &[u8], data: &[u8]) -> Result<(), (Stage, i32)> {
        let p = self.resolve(path, false).map_err(|e| (Stage::Create, e))?;
        let parent = &p[..p.iter().rposition(|&c| c == b'/').unwrap_or(0).max(1)];
        if self.nodes.get(parent) != Some(&Node::Dir) {
            return Err((Stage::Create, ENOENT));
        }
        self.nodes.insert(p, Node::File(data.to_vec()));
        Ok(())
    }
    fn error_text(&self, errno: i32, out: &mut Text) {
        out.bytes(format!("errno {errno}").as_bytes());
    }
}

/// A minimal x86-64 shared object (ELF-64, psABI): header, PT_LOAD over the file, PT_DYNAMIC, string table.
/// Segments are linked at `BIAS` (`p_vaddr` = `p_offset` + bias), so a reader that takes a virtual address for a
/// file offset finds no dynamic segment or string table.
fn elf_image(soname: Option<&[u8]>) -> Vec<u8> {
    const BIAS: u64 = 0x20_0000;
    let ndyn: usize = if soname.is_some() { 4 } else { 3 };
    let (dynoff, strtab) = (64 + 112, 64 + 112 + ndyn * 16);
    let mut strings = vec![0u8];
    if let Some(s) = soname {
        strings.extend_from_slice(s);
        strings.push(0);
    }
    let total = (strtab + strings.len()) as u64;
    let mut b = b"\x7fELF\x02\x01\x01\0\0\0\0\0\0\0\0\0".to_vec();
    for v in [3u16, 62] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&1u32.to_le_bytes());
    for v in [0u64, 64, 0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&0u32.to_le_bytes());
    for v in [64u16, 56, 2, 64, 0, 0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    for (ty, fl, off, size) in [(1u32, 4u32, 0u64, total), (2, 6, dynoff as u64, (ndyn * 16) as u64)] {
        b.extend_from_slice(&ty.to_le_bytes());
        b.extend_from_slice(&fl.to_le_bytes());
        // p_offset, p_vaddr, p_paddr, p_filesz, p_memsz, p_align
        for v in [off, off + BIAS, off + BIAS, size, size, 8] {
            b.extend_from_slice(&v.to_le_bytes());
        }
    }
    let mut dynamic = Vec::new();
    if soname.is_some() {
        dynamic.push((14u64, 1u64));
    }
    dynamic.extend([(5, strtab as u64 + BIAS), (10, strings.len() as u64), (0, 0)]);
    for (t, v) in dynamic {
        b.extend_from_slice(&t.to_le_bytes());
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&strings);
    b
}

/// A minimal ELF-32 little-endian shared object of `machine` (3 = i386, 62 = x32): gABI ELF-32 layout (52-byte
/// header, 32-byte program headers with `p_flags` after `p_memsz`, 8-byte dynamic entries). Segments are linked at
/// `ELF32_BIAS` (`p_vaddr` = `p_offset` + bias, as a non-zero base address gives), so a reader that takes a
/// virtual address for a file offset finds no dynamic segment or string table.
fn elf32_image(soname: &[u8], machine: u16) -> Vec<u8> {
    const ELF32_BIAS: u32 = 0x8000;
    let (dynoff, strtab) = (52 + 64, 52 + 64 + 4 * 8);
    let mut strings = vec![0u8];
    strings.extend_from_slice(soname);
    strings.push(0);
    let total = (strtab + strings.len()) as u32;
    let (dynva, strva) = (dynoff as u32 + ELF32_BIAS, strtab as u32 + ELF32_BIAS);
    let mut b = b"\x7fELF\x01\x01\x01\0\0\0\0\0\0\0\0\0".to_vec();
    for v in [3u16, machine] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    for v in [1u32, 0, 52, 0, 0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    for v in [52u16, 32, 2, 40, 0, 0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    // p_type, p_offset, p_vaddr, p_paddr, p_filesz, p_memsz, p_flags, p_align: PT_LOAD, then PT_DYNAMIC.
    for v in [1u32, 0, ELF32_BIAS, ELF32_BIAS, total, total, 4, 0x1000, 2, dynoff as u32, dynva, dynva, 32, 32, 6, 4] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    for v in [14u32, 1, 5, strva, 10, strings.len() as u32, 0, 0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&strings);
    b
}

fn ldconfig(fs: &mut Mem, args: &[&str]) -> Outcome {
    let a: Vec<&[u8]> = args.iter().map(|s| s.as_bytes()).collect();
    run::<X86_64, _>(b"ldconfig", &a, fs, b"test")
}

fn text(t: &Text) -> String {
    String::from_utf8_lossy(&t.buf).into_owned()
}

/// The trusted directories, present and empty.
fn base() -> Mem {
    let mut m = Mem::new();
    for d in ["/lib/x86_64-linux-gnu", "/usr/lib/x86_64-linux-gnu", "/lib", "/usr/lib"] {
        m.dir(d);
    }
    m
}

/// s31_2_i_2: comments, include globs read in sorted order, an include relative to the including file's directory,
/// the hwcap line, and the order of `-v`'s directory lines.
#[test]
fn configuration_includes_are_read_in_sorted_order_relative_to_their_file() {
    let mut m = base();
    m.file("/etc/ld.so.conf", b"/opt/a # trailing comment\ninclude /etc/conf.d/*.conf sub/x.conf\nhwcap 0 foo\n")
        .file("/etc/conf.d/b.conf", b"/opt/b\n")
        .file("/etc/conf.d/a.conf", b"   /opt/c   \n")
        .file("/etc/sub/x.conf", b"/opt/d\n");
    for d in ["/opt/a", "/opt/b", "/opt/c", "/opt/d"] {
        m.dir(d);
    }
    let o = ldconfig(&mut m, &["-N", "-X", "-v"]);
    assert_eq!(o.status, 0, "{}", text(&o.err));
    let dirs: Vec<String> = text(&o.out).lines().map(str::to_owned).collect();
    assert_eq!(
        &dirs[..4],
        ["/opt/a: (from /etc/ld.so.conf:1)", "/opt/c: (from /etc/conf.d/a.conf:1)", "/opt/b: (from /etc/conf.d/b.conf:1)", "/opt/d: (from /etc/sub/x.conf:1)"]
    );
    assert_eq!(text(&o.err), "ldconfig: /etc/ld.so.conf:3: hwcap directive ignored\n");
}

/// s31_2_i_2, libniji rule (cycles refused with a message): a file that includes itself, and two that include each
/// other, end, each directory once, with a message naming the include, exit 0.
#[test]
fn include_cycles_end_with_a_message() {
    let mut m = base();
    m.file("/etc/ld.so.conf", b"include /etc/a.conf\n/opt/x\n").file("/etc/a.conf", b"include /etc/b.conf\ninclude /etc/a.conf\n/opt/y\n").file("/etc/b.conf", b"include /etc/a.conf\n");
    m.dir("/opt/x").dir("/opt/y");
    let o = ldconfig(&mut m, &["-N", "-X", "-v"]);
    assert_eq!(o.status, 0);
    let out = text(&o.out);
    assert_eq!(out.matches("/opt/y:").count(), 1, "{out}");
    assert_eq!(out.matches("/opt/x:").count(), 1, "{out}");
    let err = text(&o.err);
    assert_eq!(err.matches("include cycle").count(), 2, "{err}");
}

/// s31_2_g_4: the soname link goes to the highest version (1.10 over 1.9 over 1.2), a second run changes nothing,
/// the development link keeps its own cache entry and is not touched.
#[test]
fn soname_links_follow_the_highest_version() {
    let mut m = base();
    m.file("/etc/ld.so.conf", b"/opt/l\n");
    for v in ["1.2", "1.9", "1.10"] {
        m.elf(&format!("/opt/l/libfoo.so.{v}"), Some("libfoo.so.1"));
    }
    m.link("/opt/l/libfoo.so", "libfoo.so.1").elf("/opt/l/libnos.so", None);
    let o = ldconfig(&mut m, &["-v"]);
    assert_eq!(o.status, 0, "{}", text(&o.err));
    assert_eq!(m.get("/opt/l/libfoo.so.1"), Some(&Node::Link(b"libfoo.so.1.10".to_vec())));
    assert!(text(&o.out).contains("\tlibfoo.so.1 -> libfoo.so.1.10 (changed)\n"));
    assert!(text(&o.out).contains("\tlibnos.so -> libnos.so\n"));
    assert_eq!(m.get("/opt/l/libfoo.so"), Some(&Node::Link(b"libfoo.so.1".to_vec())));
    // The first run scanned before it made libfoo.so.1, so the development link dangled then (the reference
    // reports it too) and only the second run's cache holds it.
    let o2 = ldconfig(&mut m, &["-v"]);
    assert!(text(&o2.out).contains("\tlibfoo.so.1 -> libfoo.so.1.10\n"), "{}", text(&o2.out));
    assert_eq!(m.get("/opt/l/libfoo.so.1"), Some(&Node::Link(b"libfoo.so.1.10".to_vec())));
    let p = text(&ldconfig(&mut m, &["-p"]).out);
    assert!(p.contains("\tlibfoo.so (libc6,x86-64) => /opt/l/libfoo.so\n"), "{p}");
    assert!(p.contains("\tlibfoo.so.1 (libc6,x86-64) => /opt/l/libfoo.so.1\n"), "{p}");
}

/// s31_2_g_4 (probe: validation/transcripts/ldconfig/{n-dev-alias,n-dev-alias-no-link,r-dev-alias-cache}.transcript;
/// GitHub issue #16): a soname link that already resolves to the winner's file is left as it is, so a `.so` alias
/// whose chain runs through the soname link (`libcmockery.so -> libcmocka.so -> libcmocka.so.0`) never makes a
/// loop. Issue #16's layout in both orders (soname link present: kept, not reported as changed; absent: the aliases
/// dangle during the scan and the link goes to the real file), plus an alias that sorts above the real file, an
/// upgrade whose alias reaches the old version, a link whose text differs from the file's name, a link at an older
/// version that must still move, and a regular file at the soname path that is the winner's own file.
#[test]
fn a_soname_link_reaching_the_winner_is_kept() {
    let issue = |with_soname_link: bool| {
        let mut m = base();
        m.elf("/opt/l/libcmocka.so.0.8.0", Some("libcmocka.so.0"));
        if with_soname_link {
            m.link("/opt/l/libcmocka.so.0", "libcmocka.so.0.8.0");
        }
        m.link("/opt/l/libcmocka.so", "libcmocka.so.0").link("/opt/l/libcmockery.so", "libcmocka.so");
        m
    };
    let mut m = issue(true);
    let o = ldconfig(&mut m, &["-n", "-v", "/opt/l"]);
    assert_eq!(o.status, 0, "{}", text(&o.err));
    assert_eq!(text(&o.out), "/opt/l: (from <cmdline>:0)\n\tlibcmocka.so.0 -> libcmockery.so\n");
    assert_eq!(m.get("/opt/l/libcmocka.so.0"), Some(&Node::Link(b"libcmocka.so.0.8.0".to_vec())));
    let mut m = issue(false);
    let o = ldconfig(&mut m, &["-n", "/opt/l"]);
    assert_eq!(o.status, 0, "{}", text(&o.err));
    assert_eq!(m.get("/opt/l/libcmocka.so.0"), Some(&Node::Link(b"libcmocka.so.0.8.0".to_vec())));
    // A second run over the result changes nothing.
    let o = ldconfig(&mut m, &["-n", "-v", "/opt/l"]);
    assert!(!text(&o.out).contains("(changed)"), "{}", text(&o.out));
    assert_eq!(m.get("/opt/l/libcmocka.so.0"), Some(&Node::Link(b"libcmocka.so.0.8.0".to_vec())));

    let mut m = base();
    m.file("/etc/ld.so.conf", b"/opt/f\n/opt/g\n/opt/h\n/opt/i\n/opt/j\n");
    // A regular file at the soname path that is the winner's own file: left alone, no message.
    m.elf("/opt/j/libtk9.so", Some("libtk9.so")).link("/opt/j/libtk9.so.0", "libtk9.so");
    // A link that reaches another file still moves to the winner.
    m.elf("/opt/i/libv.so.1.0", Some("libv.so.1")).elf("/opt/i/libv.so.1.1", Some("libv.so.1"));
    m.link("/opt/i/libv.so.1", "libv.so.1.0");
    m.elf("/opt/f/libup.so.1.0", Some("libup.so.1")).elf("/opt/f/libup.so.1.1", Some("libup.so.1"));
    m.link("/opt/f/libup.so.1", "libup.so.1.0").link("/opt/f/libup.so", "libup.so.1");
    m.link("/opt/f/libupcompat.so", "libup.so");
    m.elf("/opt/g/libsf.so.1.2", Some("libsf.so.1")).link("/opt/g/libsf.so.1", "./libsf.so.1.2");
    m.elf("/opt/h/libqvx.so.2.0", Some("libqvx.so.2")).link("/opt/h/libqvx.so.2", "libqvx.so.2.0");
    m.link("/opt/h/libqvxalt.so", "libqvx.so.2.0");
    let o = ldconfig(&mut m, &["-v"]);
    assert_eq!(o.status, 0, "{}", text(&o.err));
    let out = text(&o.out);
    for line in ["\tlibup.so.1 -> libupcompat.so\n", "\tlibsf.so.1 -> libsf.so.1.2\n", "\tlibqvx.so.2 -> libqvxalt.so\n"] {
        assert!(out.contains(line), "{line:?} in {out}");
    }
    assert_eq!(m.get("/opt/f/libup.so.1"), Some(&Node::Link(b"libup.so.1.0".to_vec())));
    assert_eq!(m.get("/opt/g/libsf.so.1"), Some(&Node::Link(b"./libsf.so.1.2".to_vec())));
    assert_eq!(m.get("/opt/h/libqvx.so.2"), Some(&Node::Link(b"libqvx.so.2.0".to_vec())));
    assert!(out.contains("\tlibv.so.1 -> libv.so.1.1 (changed)\n"), "{out}");
    assert_eq!(m.get("/opt/i/libv.so.1"), Some(&Node::Link(b"libv.so.1.1".to_vec())));
    assert!(out.contains("\tlibtk9.so -> libtk9.so.0\n"), "{out}");
    assert_eq!(text(&o.err), "", "no message for a soname path that is the winner's file");
    assert!(matches!(m.get("/opt/j/libtk9.so"), Some(Node::File(_))));
}

/// s31_2_g_4 (probe): a dangling `lib*.so*` link is removed unless `-X` or its name ends in `.so`.
#[test]
fn dangling_links_are_removed_unless_x_or_development_links() {
    let tree = || {
        let mut m = base();
        m.file("/etc/ld.so.conf", b"/opt/l\n").link("/opt/l/libx.so.1", "nowhere").link("/opt/l/libx.so", "nowhere");
        m
    };
    let mut m = tree();
    let o = ldconfig(&mut m, &["-N"]);
    assert_eq!(text(&o.err), "", "no message without -v");
    assert_eq!(m.get("/opt/l/libx.so.1"), None);
    assert!(m.get("/opt/l/libx.so").is_some());
    let mut m = tree();
    let o = ldconfig(&mut m, &["-N", "-X", "-v"]);
    assert!(m.get("/opt/l/libx.so.1").is_some());
    assert_eq!(text(&o.err).matches("Cannot stat").count(), 2, "{}", text(&o.err));
}

/// s31_2_g_5 (probe): a glibc-hwcaps subdirectory gets no link and its cache entry names the file itself.
#[test]
fn hwcaps_subdirectories_get_no_links_and_name_the_file() {
    let mut m = base();
    m.file("/etc/ld.so.conf", b"/opt/l\n").elf("/opt/l/libz.so.1.2", Some("libz.so.1")).elf("/opt/l/glibc-hwcaps/x86-64-v3/libz.so.1.2", Some("libz.so.1"));
    let o = ldconfig(&mut m, &["-v"]);
    assert_eq!(o.status, 0, "{}", text(&o.err));
    assert_eq!(m.get("/opt/l/glibc-hwcaps/x86-64-v3/libz.so.1"), None);
    let p = text(&ldconfig(&mut m, &["-p"]).out);
    assert!(p.contains("\tlibz.so.1 (libc6,x86-64, hwcap: \"x86-64-v3\") => /opt/l/glibc-hwcaps/x86-64-v3/libz.so.1.2\n"), "{p}");
    assert!(text(&o.out).contains("/opt/l/glibc-hwcaps/x86-64-v3: (hwcap: \"x86-64-v3\") (from /etc/ld.so.conf:1)\n\tlibz.so.1 -> libz.so.1.2\n"));
}

/// s31_2_i_2, LDCONFIG-LIMITS (owner ruling: no artificial caps): a 3 MiB `ld.so.conf` (old cap 1 MiB, the rest
/// was ignored silently) is read whole: its last line's directory is scanned and its library gets its link.
#[test]
fn a_large_configuration_is_read_to_its_last_line() {
    let mut m = base();
    let mut conf = Vec::new();
    while conf.len() < 3 << 20 {
        conf.extend_from_slice(b"# filler comment line of a large configuration file, ignored by the reader\n");
    }
    conf.extend_from_slice(b"/opt/last\n");
    m.file("/etc/ld.so.conf", &conf).elf("/opt/last/liblast.so.1.0", Some("liblast.so.1"));
    let o = ldconfig(&mut m, &["-N", "-v"]);
    assert_eq!(o.status, 0, "{}", text(&o.err));
    assert!(text(&o.out).starts_with("/opt/last: (from /etc/ld.so.conf:"), "a_large_configuration: the last line is read: {}", text(&o.out));
    assert_eq!(m.get("/opt/last/liblast.so.1"), Some(&Node::Link(b"liblast.so.1.0".to_vec())), "a_large_configuration: the last directory is scanned");
}

/// s31_2_i_2, LDCONFIG-LIMITS (ruling #5594: running out of memory is a clean error): a configuration file the file
/// system cannot read for want of memory ends the run with `out of memory`, exit 1; it is not skipped like a missing
/// file.
#[test]
fn a_configuration_read_out_of_memory_ends_the_run() {
    let mut m = base();
    m.file("/etc/ld.so.conf", b"include /etc/enomem.conf\n/opt/a\n").file("/etc/enomem.conf", b"/opt/b\n").dir("/opt/a").dir("/opt/b");
    let o = ldconfig(&mut m, &["-N", "-X"]);
    assert_eq!((o.status, text(&o.err)), (1, "ldconfig: out of memory\n".to_string()), "a_configuration_read_out_of_memory");
}

/// s31_2_i_2, s31_2_i_3, LDCONFIG-LIMITS: 70000 configured directories (old cap 65536, the rest ignored with a
/// message) are all kept, in order; one named again at the end is still found as a duplicate of its first line.
#[test]
fn many_configured_directories_are_all_kept() {
    const N: usize = 70_000;
    let mut m = base();
    let mut conf = Vec::new();
    for i in 0..N {
        let d = format!("/d/{i}");
        m.dir(&d);
        conf.extend_from_slice(d.as_bytes());
        conf.push(b'\n');
    }
    conf.extend_from_slice(b"/d/0\n");
    m.file("/etc/ld.so.conf", &conf);
    let o = ldconfig(&mut m, &["-N", "-X", "-v"]);
    assert_eq!(o.status, 0, "{}", text(&o.err));
    let out = text(&o.out);
    let dirs: Vec<&str> = out.lines().filter(|l| l.starts_with("/d/")).collect();
    assert_eq!(dirs.len(), N, "many_configured_directories: every directory kept");
    assert_eq!(dirs[N - 1], format!("/d/{}: (from /etc/ld.so.conf:{N})", N - 1));
    assert_eq!(
        text(&o.err),
        format!("ldconfig: Path `/d/0' given more than once\n(from /etc/ld.so.conf:{} and /etc/ld.so.conf:1)\n", N + 1),
        "many_configured_directories: the duplicate is found, nothing is ignored"
    );
}

/// s31_2_g_1, s31_2_g_5, LDCONFIG-LIMITS: 100 glibc-hwcaps subdirectories (old cap 64 distinct names, the cache was
/// not written) all go into the cache, each with its name.
#[test]
fn many_hwcaps_subdirectories_reach_the_cache() {
    let mut m = base();
    m.file("/etc/ld.so.conf", b"/opt/h\n").elf("/opt/h/libh.so.1.0", Some("libh.so.1"));
    for i in 0..100 {
        m.elf(&format!("/opt/h/glibc-hwcaps/level{i}/libh.so.1.0"), Some("libh.so.1"));
    }
    let o = ldconfig(&mut m, &[]);
    assert_eq!(o.status, 0, "{}", text(&o.err));
    let p = text(&ldconfig(&mut m, &["-p"]).out);
    for i in 0..100 {
        let line = format!("\tlibh.so.1 (libc6,x86-64, hwcap: \"level{i}\") => /opt/h/glibc-hwcaps/level{i}/libh.so.1.0\n");
        assert!(p.contains(&line), "many_hwcaps_subdirectories: level{i} in the cache: {p}");
    }
    assert!(p.starts_with("101 libs found in cache"), "{p}");
}

/// s31_2_g_3 (libniji rule): the same tree listed in the opposite order gives the same output and the same cache.
#[test]
fn listing_order_changes_nothing() {
    let mut a = base();
    a.file("/etc/ld.so.conf", b"include /etc/d/*\n").file("/etc/d/1", b"/opt/l\n").file("/etc/d/2", b"/opt/m\n");
    for (d, n) in [("l", "aa"), ("l", "bb"), ("m", "cc"), ("m", "aa")] {
        for v in ["1.0", "1.1"] {
            a.elf(&format!("/opt/{d}/lib{n}.so.{v}"), Some(&format!("lib{n}.so.1")));
        }
    }
    let mut b = a.clone();
    b.reverse = true;
    let (oa, ob) = (ldconfig(&mut a, &["-v"]), ldconfig(&mut b, &["-v"]));
    assert_eq!((text(&oa.out), text(&oa.err), oa.status), (text(&ob.out), text(&ob.err), ob.status));
    assert_eq!(a.get("/etc/ld.so.cache"), b.get("/etc/ld.so.cache"));
    assert!(a.get("/etc/ld.so.cache").is_some());
}

/// s31_2_c_1, s31_2_g_4 (probe: validation/transcripts/ldconfig/foreign-class.transcript): i386 and x32 objects are
/// linked and recorded with their own type's flags. Same soname, different types in one directory (the reference's
/// warning and flags there follow its directory order; pending record UTIL-LDCONFIG-5-1): the highest version wins
/// the link and the entry carries the winner's type, and the other-type file is reported, in either listing order.
#[test]
fn foreign_class_objects_are_recorded_with_their_type() {
    let mut a = base();
    a.file("/etc/ld.so.conf", b"/opt/i\n/opt/x\n/opt/m\n/opt/k\n");
    a.file("/opt/i/libi.so.2.0", &elf32_image(b"libi.so.2", 3)).file("/opt/x/libx.so.3.0", &elf32_image(b"libx.so.3", 62));
    a.elf("/opt/m/libmix.so.1.0", Some("libmix.so.1")).file("/opt/m/libmix.so.1.5", &elf32_image(b"libmix.so.1", 3));
    a.elf("/opt/k/libmox.so.1.5", Some("libmox.so.1")).file("/opt/k/libmox.so.1.0", &elf32_image(b"libmox.so.1", 3));
    let mut b = a.clone();
    b.reverse = true;
    for m in [&mut a, &mut b] {
        let o = ldconfig(m, &["-v"]);
        assert_eq!(o.status, 0, "{}", text(&o.err));
        assert_eq!(m.get("/opt/i/libi.so.2"), Some(&Node::Link(b"libi.so.2.0".to_vec())));
        assert_eq!(m.get("/opt/x/libx.so.3"), Some(&Node::Link(b"libx.so.3.0".to_vec())));
        assert_eq!(m.get("/opt/m/libmix.so.1"), Some(&Node::Link(b"libmix.so.1.5".to_vec())));
        assert_eq!(m.get("/opt/k/libmox.so.1"), Some(&Node::Link(b"libmox.so.1.5".to_vec())));
        assert_eq!(
            text(&o.err),
            "ldconfig: libraries libmix.so.1.0 and libmix.so.1.5 in directory /opt/m have same soname but different type.\n\
             ldconfig: libraries libmox.so.1.0 and libmox.so.1.5 in directory /opt/k have same soname but different type.\n"
        );
        let p = text(&ldconfig(m, &["-p"]).out);
        for line in [
            "\tlibi.so.2 (libc6) => /opt/i/libi.so.2\n",
            "\tlibx.so.3 (libc6,x32) => /opt/x/libx.so.3\n",
            "\tlibmix.so.1 (libc6) => /opt/m/libmix.so.1\n",
            "\tlibmox.so.1 (libc6,x86-64) => /opt/k/libmox.so.1\n",
        ] {
            assert!(p.contains(line), "{line:?} not in {p}");
        }
    }
}

/// s31_2_i_4: under `-r` a relative include is refused, exit 1, nothing written.
#[test]
fn relative_include_under_a_root_is_refused() {
    let mut m = base();
    m.file("/etc/ld.so.conf", b"include x.conf\n");
    let o = ldconfig(&mut m, &["-r", "/"]);
    assert_eq!(o.status, 1);
    assert_eq!(text(&o.err), "ldconfig: need absolute file name for configuration file when using -r\n");
    assert!(m.get("/etc/ld.so.cache").is_none());
}

/// no spec clause (robustness, the brief's fuzz proof): 3000 seeded random configurations built from the syntax's
/// own pieces (includes of each other and of globs, comments, blanks, odd bytes) all end with status 0 or 1, never
/// panic, and the same tree run twice gives the same output and the same files (a second run on the FIRST run's
/// result may differ legitimately: a glob include can read the cache that run wrote). Seeded, so a failure
/// reproduces exactly.
#[test]
fn random_configurations_end_cleanly() {
    const PIECES: [&[u8]; 16] = [
        b"include ", b"/etc/c", b"*", b"?", b"[a-c]", b"[!", b"]", b"/opt/l", b"#", b"\n", b" ", b"\t", b"..", b"/", b"\\", b"\xff",
    ];
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..3000 {
        let mut m = base();
        m.elf("/opt/l/libq.so.1.0", Some("libq.so.1"));
        for f in ["/etc/ld.so.conf", "/etc/ca", "/etc/cb", "/etc/c[x]"] {
            let mut body = Vec::new();
            for _ in 0..(next() % 24) {
                body.extend_from_slice(PIECES[(next() % 16) as usize]);
            }
            m.file(f, &body);
        }
        let mut twin = m.clone();
        let o = ldconfig(&mut m, &["-v"]);
        assert!(o.status <= 1, "status {}: {}", o.status, text(&o.err));
        let again = ldconfig(&mut twin, &["-v"]);
        assert_eq!((text(&again.out), text(&again.err), again.status), (text(&o.out), text(&o.err), o.status));
        assert_eq!(twin.get("/etc/ld.so.cache"), m.get("/etc/ld.so.cache"));
        assert_eq!(twin.get("/opt/l/libq.so.1"), m.get("/opt/l/libq.so.1"));
    }
}
