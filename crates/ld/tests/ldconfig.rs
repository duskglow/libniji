//! `ldconfig`'s engine (spec §31.2; plan/utilities.md UTIL-3) over an in-memory file system: the configuration
//! reader, the link rule, determinism and a seeded fuzz loop over configuration files. The transcripts
//! (validation/transcripts/ldconfig/) compare the same engine with the reference; these tests reach what a
//! transcript cannot: include cycles (a deliberate difference), listing order, and many random inputs.

use liella_ld::arch::X86_64;
use liella_ld::ldconfig::{run, Fs, Kind, Meta, Outcome, Stage, Text};
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
fn elf_image(soname: Option<&[u8]>) -> Vec<u8> {
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
        for v in [off, off, off, size, size, 8] {
            b.extend_from_slice(&v.to_le_bytes());
        }
    }
    let mut dynamic = Vec::new();
    if soname.is_some() {
        dynamic.push((14u64, 1u64));
    }
    dynamic.extend([(5, strtab as u64), (10, strings.len() as u64), (0, 0)]);
    for (t, v) in dynamic {
        b.extend_from_slice(&t.to_le_bytes());
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

/// s31_2_i_2, Liella rule (cycles refused with a message): a file that includes itself, and two that include each
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
    let o2 = ldconfig(&mut m, &["-v", "-p"]);
    let p = text(&o2.out);
    assert!(p.contains("\tlibfoo.so (libc6,x86-64) => /opt/l/libfoo.so\n"), "{p}");
    assert!(p.contains("\tlibfoo.so.1 (libc6,x86-64) => /opt/l/libfoo.so.1\n"), "{p}");
    let o3 = ldconfig(&mut m, &["-v"]);
    assert!(text(&o3.out).contains("\tlibfoo.so.1 -> libfoo.so.1.10\n"), "{}", text(&o3.out));
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

/// s31_2_g_3 (Liella rule): the same tree listed in the opposite order gives the same output and the same cache.
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

/// No spec clause (robustness, the brief's fuzz proof): 3000 seeded random configurations built from the syntax's
/// own pieces (includes of each other and of globs, comments, blanks, odd bytes) all end with status 0 or 1, never
/// panic, and a second run on the result gives the same output. Seeded, so a failure reproduces exactly.
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
        let o = ldconfig(&mut m, &["-v"]);
        assert!(o.status <= 1, "status {}: {}", o.status, text(&o.err));
        let again = ldconfig(&mut m.clone(), &["-v"]);
        assert_eq!(text(&again.err), text(&o.err));
    }
}
