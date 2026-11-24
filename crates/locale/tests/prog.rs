//! The `localedef` command (`niji_locale::prog`) over an in-memory file system: the options, the
//! input search paths, the directory form and the messages and statuses (spec §31.8); the `locales`
//! package's commands `locale-gen`, `update-locale` and `validlocale` (`niji_locale::cmds`, §31.14).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::as_conversions, missing_docs)]

use std::collections::BTreeMap;

use niji_locale::prog::{run, Host, Outcome};
use niji_locale::{Files, Kind, TryVec};

/// Files and directories by path; `fail_mkdir` makes every `mkdir` fail with that errno.
#[derive(Default)]
struct Mem {
    files: BTreeMap<Vec<u8>, Vec<u8>>,
    dirs: Vec<Vec<u8>>,
    fail_mkdir: Option<i32>,
    /// The standard input (`None`: reading it fails with EBADF).
    stdin: Option<Vec<u8>>,
    /// The directories locked, in order.
    locked: Vec<Vec<u8>>,
    i18npath: Option<Vec<u8>>,
    /// `DEFAULTCHARSET`'s value (`validlocale`).
    defaultcharset: Option<Vec<u8>>,
    /// `PATH`'s value (`validlocale`'s usage line).
    path: Option<Vec<u8>>,
}

impl Host for Mem {
    fn read(&mut self, path: &[u8], _max: usize) -> Result<TryVec<u8>, i32> {
        self.files.get(path).map(|d| TryVec::from_slice(d).unwrap()).ok_or(2)
    }
    fn mkdir(&mut self, path: &[u8]) -> Result<(), i32> {
        if let Some(e) = self.fail_mkdir {
            return Err(e);
        }
        if self.dirs.iter().any(|d| d == path) {
            return Err(17);
        }
        self.dirs.push(path.to_vec());
        Ok(())
    }
    fn write(&mut self, path: &[u8], data: &[u8]) -> Result<(), i32> {
        self.files.insert(path.to_vec(), data.to_vec());
        Ok(())
    }
    fn read_stdin(&mut self, _max: usize) -> Result<TryVec<u8>, i32> {
        self.stdin.take().map(|d| TryVec::from_slice(&d).unwrap()).ok_or(9)
    }
    fn lock(&mut self, dir: &[u8]) -> Result<(), i32> {
        self.locked.push(dir.to_vec());
        Ok(())
    }
    fn i18npath(&self) -> Option<&[u8]> {
        self.i18npath.as_deref()
    }
    fn remove(&mut self, path: &[u8]) -> Result<(), i32> {
        self.files.remove(path).map(|_| ()).ok_or(2)
    }
    fn exists(&mut self, path: &[u8]) -> bool {
        self.files.contains_key(path) || self.dirs.iter().any(|d| d == path)
    }
    fn env(&self, name: &[u8]) -> Option<&[u8]> {
        match name {
            b"DEFAULTCHARSET" => self.defaultcharset.as_deref(),
            b"PATH" => self.path.as_deref(),
            _ => None,
        }
    }
    fn error_text(&self, errno: i32) -> &'static [u8] {
        match errno {
            2 => b"No such file or directory",
            13 => b"Permission denied",
            _ => b"Unknown error",
        }
    }
}

/// A charmap of the 128 ASCII characters.
fn ascii_charmap() -> Vec<u8> {
    let mut s = String::from("<code_set_name> MINI\n<mb_cur_max> 1\n<mb_cur_min> 1\nCHARMAP\n");
    for c in 0..128u32 {
        s.push_str(&format!("<U{c:04X}> /x{c:02x}\n"));
    }
    s.push_str("END CHARMAP\n");
    s.into_bytes()
}

/// A source that defines two categories: `LC_NUMERIC` and `LC_MESSAGES`.
const MINI: &[u8] = b"LC_NUMERIC\ndecimal_point \"<U002C>\"\nthousands_sep \"\"\ngrouping -1\nEND LC_NUMERIC\n\
LC_MESSAGES\nyesexpr \"<U005E><U0079>\"\nnoexpr \"<U005E><U006E>\"\nyesstr \"<U0079>\"\nnostr \"<U006E>\"\nEND LC_MESSAGES\n";

fn host() -> Mem {
    let mut m = Mem::default();
    m.files.insert(b"/usr/share/i18n/charmaps/MINI".to_vec(), ascii_charmap());
    m.files.insert(b"/usr/share/i18n/locales/mini".to_vec(), MINI.to_vec());
    m
}

fn go(m: &mut Mem, args: &[&str]) -> Outcome {
    let args: Vec<&[u8]> = args.iter().map(|a| a.as_bytes()).collect();
    run(&args, m)
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// The library's own compile of the same input (the files the command must write unchanged).
struct Plain<'a>(&'a Mem);

impl Files for Plain<'_> {
    fn read(&mut self, kind: Kind, name: &[u8]) -> niji_locale::Result<TryVec<u8>> {
        let dir: &[u8] = if kind == Kind::Locale { b"/usr/share/i18n/locales/" } else { b"/usr/share/i18n/charmaps/" };
        let path = [dir, name].concat();
        Ok(TryVec::from_slice(self.0.files.get(&path).ok_or(niji_locale::Error::missing("none"))?).unwrap())
    }
}

/// s31_8_g_1, s31_8_g_3: a NAME with `/` gets the directory form: the directory, `LC_MESSAGES/`, and all twelve
/// category files, a defined one the library's compiled file; a category the source lacks is a warning, status 1,
/// and its file holds the default.
#[test]
fn directory_form_writes_the_defined_categories() {
    let mut m = host();
    let o = go(&mut m, &["-i", "mini", "-c", "-f", "MINI", "/out/mini.MINI"]);
    assert_eq!(o.status, 1, "{}", text(&o.err));
    assert!(o.out.is_empty());
    let err = text(&o.err);
    assert_eq!(err.lines().count(), 10, "{err}");
    assert!(err.starts_with("[warning] No definition for LC_CTYPE category found\n[warning] No definition for LC_TIME category found\n"), "{err}");
    assert_eq!(m.dirs, [b"/out/mini.MINI".to_vec(), b"/out/mini.MINI/LC_MESSAGES".to_vec()]);
    let compiled = niji_locale::compile(b"mini", b"MINI", &mut Plain(&host())).unwrap();
    let numeric = m.files.get(&b"/out/mini.MINI/LC_NUMERIC"[..]).expect("LC_NUMERIC written");
    assert_eq!(numeric.as_slice(), compiled[1].as_ref().unwrap().as_slice());
    let messages = m.files.get(&b"/out/mini.MINI/LC_MESSAGES/SYS_LC_MESSAGES"[..]).expect("SYS_LC_MESSAGES written");
    assert_eq!(messages.as_slice(), compiled[5].as_ref().unwrap().as_slice());
    assert_eq!(m.files.len(), 14, "all twelve category files are added");
    let defaults = niji_locale::compile_output(b"mini", b"MINI", &mut Plain(&host())).unwrap();
    let time = m.files.get(&b"/out/mini.MINI/LC_TIME"[..]).expect("LC_TIME written with its default");
    assert_eq!(time.as_slice(), defaults.files[2].as_ref().unwrap().as_slice());
    // A second run over the existing directory replaces the files (an existing directory is no error).
    let again = go(&mut m, &["-i", "mini", "-fMINI", "--quiet", "/out/mini.MINI/"]);
    assert_eq!((again.status, text(&again.err)), (0, String::new()));
}

/// s31_8_f_1: a missing source is one `[error]` line and status 4; only the output directory is made (first).
#[test]
fn missing_source_is_status_4() {
    let mut m = host();
    let o = go(&mut m, &["-i", "nosuch", "-f", "MINI", "/out/x"]);
    assert_eq!(text(&o.err), "[error] cannot open locale definition file `nosuch': No such file or directory\n");
    assert_eq!(o.status, 4);
    assert!(m.dirs == [b"/out/x".to_vec()] && m.files.len() == 2, "only the output directory is made");
}

/// s31_8_f_1: a missing charmap: its `[error]` line and the default charmap in its place; without -c the no-output
/// line, status 4, nothing written.
#[test]
fn missing_charmap_is_status_4() {
    let mut m = host();
    let o = go(&mut m, &["--inputfile=mini", "--charmap", "NOSUCH", "/out/x"]);
    assert_eq!(
        text(&o.err),
        "[error] character map file `NOSUCH' not found: No such file or directory\n[error] default character map file `ANSI_X3.4-1968' not found: No such file or directory\n[error] no output file produced because errors were issued\n"
    );
    assert_eq!(o.status, 4);
    assert!(m.dirs == [b"/out/x".to_vec()] && m.files.len() == 2, "only the output directory is made");
    // The default charmap stands in: the source is compiled, but only -c writes it.
    m.files.insert(b"/usr/share/i18n/charmaps/ANSI_X3.4-1968".to_vec(), ascii_charmap());
    let o = go(&mut m, &["-i", "mini", "-f", "NOSUCH", "--quiet", "/out/x"]);
    assert_eq!(text(&o.err), "[error] character map file `NOSUCH' not found: No such file or directory\n[error] no output file produced because errors were issued\n");
    assert_eq!(o.status, 4);
    assert_eq!(m.files.len(), 3);
    let o = go(&mut m, &["-i", "mini", "-f", "NOSUCH", "--quiet", "-c", "/out/x"]);
    assert_eq!(text(&o.err), "[error] character map file `NOSUCH' not found: No such file or directory\n");
    assert_eq!(o.status, 0);
    assert!(m.files.contains_key(&b"/out/x/LC_NUMERIC"[..]));
}

/// A gzip file of one stored deflate block.
fn gzip_stored(data: &[u8]) -> Vec<u8> {
    let mut g = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff];
    g.push(1);
    let len = data.len() as u16;
    g.extend_from_slice(&len.to_le_bytes());
    g.extend_from_slice(&(!len).to_le_bytes());
    g.extend_from_slice(data);
    g.extend_from_slice(&niji::charmap::inflate::crc32(data).to_le_bytes());
    g.extend_from_slice(&(data.len() as u32).to_le_bytes());
    g
}

/// s31_8_i_2: a charmap named without `/` is found in the charmap directory, as `<name>.gz` when only the compressed
/// file is there, and a source named with `/` is read from that path.
#[test]
fn charmap_search_takes_the_compressed_file() {
    let mut m = host();
    let plain = m.files.remove(&b"/usr/share/i18n/charmaps/MINI"[..]).unwrap();
    m.files.insert(b"/usr/share/i18n/charmaps/MINI.gz".to_vec(), gzip_stored(&plain));
    m.files.insert(b"./src/mini".to_vec(), MINI.to_vec());
    let o = go(&mut m, &["-i", "./src/mini", "-f", "MINI", "--quiet", "/out/gz"]);
    assert_eq!((o.status, text(&o.err)), (0, String::new()));
    assert!(m.files.contains_key(&b"/out/gz/LC_NUMERIC"[..]));
}

/// s31_8_g_1, s31_8_g_4: a NAME without `/` with `--no-archive` goes under the locale path (after `--prefix`) with its codeset
/// normalized; without it the locale goes into the archive there, under the lock of its directory.
#[test]
fn name_without_slash() {
    let mut m = host();
    let o = go(&mut m, &["-i", "mini", "-f", "MINI", "--no-archive", "--prefix=/root", "--quiet", "mini.MINI"]);
    assert_eq!((o.status, text(&o.err)), (0, String::new()));
    assert!(m.files.contains_key(&b"/root/usr/lib/locale/mini.mini/LC_NUMERIC"[..]));
    let mut m = host();
    let o = go(&mut m, &["-i", "mini", "-f", "MINI", "--quiet", "mini.MINI"]);
    assert_eq!((o.status, text(&o.err)), (0, String::new()));
    assert!(m.dirs.is_empty(), "no directory is made for the archive form");
    assert_eq!(m.locked, [b"/usr/lib/locale".to_vec()]);
    assert!(m.files.contains_key(&b"/usr/lib/locale/locale-archive"[..]));
    assert_eq!(text(&go(&mut m, &["--list-archive"]).out), "mini.mini\n");
}

/// The archive named `path` in `m`, parsed.
fn archive_at(m: &Mem, path: &str) -> niji_locale::archive::Archive {
    niji_locale::archive::Archive::parse(m.files.get(path.as_bytes()).expect("archive written")).unwrap()
}

/// s31_8_g_4: a NAME without a codeset is stored as given and with the charmap's codeset (normalized) before any
/// `@modifier`, the two names one locale; a second compile of a stored name replaces it without a message.
#[test]
fn archive_names_take_the_charmaps_codeset() {
    let mut m = host();
    let o = go(&mut m, &["-i", "mini", "-f", "MINI", "--quiet", "--prefix=/p", "zz_ZZ@euro"]);
    assert_eq!((o.status, text(&o.err)), (0, String::new()));
    assert_eq!(text(&go(&mut m, &["--prefix=/p", "--list-archive"]).out), "zz_ZZ.mini@euro\nzz_ZZ@euro\n");
    let first = m.files.get(&b"/p/usr/lib/locale/locale-archive"[..]).unwrap().clone();
    let o = go(&mut m, &["-i", "mini", "-f", "MINI", "--quiet", "--prefix=/p", "zz_ZZ@euro"]);
    assert_eq!((o.status, text(&o.err), text(&o.out)), (0, String::new(), String::new()));
    assert_eq!(m.files.get(&b"/p/usr/lib/locale/locale-archive"[..]).unwrap(), &first, "a replaced locale leaves the same archive");
    assert_eq!(archive_at(&m, "/p/usr/lib/locale/locale-archive").names().unwrap().len(), 2);
}

/// s31_8_g_2: `--list-archive` lists the names sorted; a FILE operand names the archive, and a missing one is a
/// message and status 1; a missing default archive lists nothing, status 0.
#[test]
fn list_archive() {
    let mut m = host();
    let o = go(&mut m, &["--list-archive"]);
    assert_eq!((o.status, text(&o.out), text(&o.err)), (0, String::new(), String::new()));
    let o = go(&mut m, &["--list-archive", "/nosuch/archive", "ignored"]);
    assert_eq!((o.status, text(&o.err)), (1, "cannot open locale archive \"/nosuch/archive\": No such file or directory\n".to_string()));
    for name in ["b_B.MINI", "a_A.MINI", "c_C.MINI"] {
        assert_eq!(go(&mut m, &["-i", "mini", "-f", "MINI", "--quiet", name]).status, 0);
    }
    let o = go(&mut m, &["--list-archive", "/usr/lib/locale/locale-archive"]);
    assert_eq!((o.status, text(&o.out)), (0, "a_A.mini\nb_B.mini\nc_C.mini\n".to_string()));
}

/// s31_8_g_4: `--delete-from-archive` removes exact names; a name not there is a message and status 0; the other
/// locales stay readable.
#[test]
fn delete_from_archive() {
    let mut m = host();
    for name in ["a_A.MINI", "b_B.MINI"] {
        assert_eq!(go(&mut m, &["-i", "mini", "-f", "MINI", "--quiet", name]).status, 0);
    }
    let o = go(&mut m, &["--delete-from-archive", "a_A.MINI", "a_A.mini"]);
    assert_eq!((o.status, text(&o.err)), (0, "locale \"a_A.MINI\" not in archive\n".to_string()));
    assert_eq!(text(&go(&mut m, &["--list-archive"]).out), "b_B.mini\n");
    let a = archive_at(&m, "/usr/lib/locale/locale-archive");
    assert!(a.contains(b"b_B.mini") && !a.contains(b"a_A.mini"));
    assert_eq!(go(&mut m, &["--delete-from-archive"]).status, 0);
}

/// s31_8_g_4: `--add-to-archive` stores a directory-form locale under its last component, normalized, with an
/// `Adding` line; an existing name is a message and status 255 unless `--replace`; a missing directory is a
/// message and is skipped (status 0).
#[test]
fn add_to_archive() {
    let mut m = host();
    assert_eq!(go(&mut m, &["-i", "mini", "-f", "MINI", "--quiet", "/out/fr_FR.UTF-8"]).status, 0);
    let o = go(&mut m, &["--add-to-archive", "/out/fr_FR.UTF-8/"]);
    assert_eq!((o.status, text(&o.out), text(&o.err)), (0, "Adding /out/fr_FR.UTF-8/\n".to_string(), String::new()));
    assert_eq!(text(&go(&mut m, &["--list-archive"]).out), "fr_FR.utf8\n");
    let o = go(&mut m, &["--add-to-archive", "/out/fr_FR.UTF-8"]);
    assert_eq!((o.status, text(&o.err)), (255, "locale 'fr_FR.utf8' already exists\n".to_string()));
    assert_eq!(go(&mut m, &["--add-to-archive", "--replace", "/out/fr_FR.UTF-8"]).status, 0);
    let o = go(&mut m, &["--add-to-archive", "/out/none"]);
    assert_eq!((o.status, text(&o.out), text(&o.err)), (0, "Adding /out/none\n".to_string(), "stat of \"/out/none\" failed: No such file or directory: ignored\n".to_string()));
    // The added locale is the directory's files: the same archive as compiling into it.
    let added = m.files.get(&b"/usr/lib/locale/locale-archive"[..]).unwrap().clone();
    let mut direct = host();
    assert_eq!(go(&mut direct, &["-i", "mini", "-f", "MINI", "--quiet", "fr_FR.UTF-8"]).status, 0);
    assert_eq!(direct.files.get(&b"/usr/lib/locale/locale-archive"[..]).unwrap(), &added);
}

/// s31_8_i_3, s31_8_f_2: without `-i` the source is the standard input, named `<stdin>` in messages; a line outside any
/// category is an error the compile goes past (status 1, the locale written, with or without -c).
#[test]
fn source_from_stdin_and_lines_outside_a_category() {
    let mut m = host();
    m.stdin = Some(MINI.to_vec());
    let o = go(&mut m, &["-f", "MINI", "--quiet", "/out/s"]);
    assert_eq!((o.status, text(&o.err)), (0, String::new()));
    assert!(m.files.contains_key(&b"/out/s/LC_NUMERIC"[..]));
    m.stdin = Some([&b"stray line\n"[..], MINI].concat());
    let o = go(&mut m, &["-f", "MINI", "--quiet", "/out/t"]);
    assert_eq!((o.status, text(&o.err)), (1, "<stdin>:1: syntax error: not inside a locale definition section\n".to_string()));
    assert_eq!(m.files.get(&b"/out/t/LC_NUMERIC"[..]), m.files.get(&b"/out/s/LC_NUMERIC"[..]));
    // No standard input: an error, status 4.
    let o = go(&mut m, &["-f", "MINI", "/out/u"]);
    assert_eq!(o.status, 4);
    assert!(!m.files.contains_key(&b"/out/u/LC_NUMERIC"[..]));
}

/// s31_8_f_2: an error inside a category names the category; without -c nothing is written (status 4); with -c the
/// locale is written, that category with its default, status 1.
#[test]
fn category_error_needs_force() {
    let messages = MINI.windows(11).position(|w| w == b"LC_MESSAGES").unwrap();
    let bad = [&b"LC_NUMERIC\ndecimal_point \"<U002E>\" junk junk\nEND LC_NUMERIC\n"[..], &MINI[messages..]].concat();
    let mut m = host();
    m.files.insert(b"/usr/share/i18n/locales/bad".to_vec(), bad);
    let o = go(&mut m, &["-i", "bad", "-f", "MINI", "--quiet", "/out/b"]);
    let err = text(&o.err);
    assert_eq!(o.status, 4, "{err}");
    assert!(err.starts_with("bad:2: LC_NUMERIC: syntax error") && err.ends_with("[error] no output file produced because errors were issued\n"), "{err}");
    assert!(!m.files.contains_key(&b"/out/b/LC_NUMERIC"[..]));
    let o = go(&mut m, &["-i", "bad", "-f", "MINI", "--quiet", "-c", "/out/b"]);
    assert_eq!(o.status, 1, "{}", text(&o.err));
    let defaults = niji_locale::compile_output(b"mini", b"MINI", &mut Plain(&host())).unwrap();
    let numeric = m.files.get(&b"/out/b/LC_NUMERIC"[..]).expect("LC_NUMERIC written under -c");
    let posix = niji_locale::compile_output(b"none", b"MINI", &mut Plain(&{
        let mut h = host();
        h.files.insert(b"/usr/share/i18n/locales/none".to_vec(), b"LC_MESSAGES\nEND LC_MESSAGES\n".to_vec());
        h
    }))
    .unwrap();
    assert_eq!(numeric.as_slice(), posix.files[1].as_ref().unwrap().as_slice(), "the failed category has its default");
    assert_ne!(numeric.as_slice(), defaults.files[1].as_ref().unwrap().as_slice());
    assert!(m.files.contains_key(&b"/out/b/LC_MESSAGES/SYS_LC_MESSAGES"[..]));
}

/// s31_8_g_1: an output directory that cannot be made is an `[error]` line naming it, status 4.
#[test]
fn unwritable_output_is_status_4() {
    let mut m = host();
    m.fail_mkdir = Some(13);
    let o = go(&mut m, &["-i", "mini", "-f", "MINI", "-c", "/ro/x"]);
    assert_eq!(o.status, 4);
    assert!(text(&o.err).ends_with("[error] cannot write output files to `/ro/x': Permission denied\n"), "{}", text(&o.err));
    assert_eq!(m.files.len(), 2);
}

/// s31_8_i_1, s31_1_c_2: the argp-style options: help, usage and version on stdout with status 0; an unknown, an
/// ambiguous or an incomplete option is a message, the `Try` line and status 4; a wrong operand count is the `Try`
/// line on stdout.
#[test]
fn options() {
    let mut m = host();
    let help = go(&mut m, &["--help"]);
    assert_eq!(help.status, 0);
    assert!(text(&help.out).starts_with("Usage: localedef [OPTION...] NAME\n"));
    assert!(text(&help.out).ends_with("\t\t       locale path    : /usr/lib/locale:/usr/share/i18n\n"));
    assert_eq!(text(&go(&mut m, &["-?"]).out), text(&help.out));
    assert!(text(&go(&mut m, &["--usage"]).out).starts_with("Usage: localedef [-cv?V] [-f FILE]"));
    assert_eq!(text(&go(&mut m, &["--vers"]).out), "localedef (libniji) 2.39\n");
    let try_line = "Try `localedef --help' or `localedef --usage' for more information.\n";
    for (args, msg) in [
        (&["-x"][..], "localedef: invalid option -- 'x'\n"),
        (&["--bogus"][..], "localedef: unrecognized option '--bogus'\n"),
        (&["--no", "x/y"][..], "localedef: option '--no' is ambiguous; possibilities: '--no-hard-links' '--no-warnings' '--no-archive'\n"),
        (&["-i"][..], "localedef: option requires an argument -- 'i'\n"),
        (&["--force=1", "x/y"][..], "localedef: option '--force' doesn't allow an argument\n"),
    ] {
        let o = go(&mut m, args);
        assert_eq!((o.status, text(&o.err)), (4, format!("{msg}{try_line}")), "{args:?}");
    }
    for args in [&[][..], &["a/b", "c/d"][..]] {
        let o = go(&mut m, args);
        assert_eq!((o.status, text(&o.out), text(&o.err)), (4, try_line.to_string(), String::new()), "{args:?}");
    }
    assert!(m.dirs.is_empty());
}

/// MINI's source with `decimal_point` set to `c` (a letter that marks which file was read).
fn marked(c: char) -> Vec<u8> {
    format!("LC_NUMERIC\ndecimal_point \"{c}\"\nthousands_sep \"\"\ngrouping -1\nEND LC_NUMERIC\n").into_bytes()
}

/// The marker letter of the compiled `LC_NUMERIC` at `dir` (its decimal point string).
fn marker(m: &Mem, dir: &str) -> Option<u8> {
    let f = m.files.get(format!("{dir}/LC_NUMERIC").as_bytes())?;
    (b'A'..=b'Z').find(|&c| f.windows(2).any(|w| w == [c, 0]))
}

/// s31_8_i_4: a `-i` or `copy` name is tried from the working directory, then for each non-empty `I18NPATH` entry
/// in order as `<entry>/locales/<name>` and `<entry>/<name>`, then in the default directory; a relative name
/// with `/` skips the default directory; none readable: status 4. (`--quiet` keeps the partial sources at status 0.)
#[test]
fn source_search_order() {
    let mut m = host();
    m.i18npath = Some(b"/e0::/e1".to_vec());
    for (path, c) in [("tsrc", 'A'), ("/e1/locales/tsrc", 'B'), ("/e1/tsrc", 'C'), ("/usr/share/i18n/locales/tsrc", 'D')] {
        m.files.insert(path.as_bytes().to_vec(), marked(c));
    }
    for (gone, want) in [("", b'A'), ("tsrc", b'B'), ("/e1/locales/tsrc", b'C'), ("/e1/tsrc", b'D')] {
        m.files.remove(gone.as_bytes());
        let o = go(&mut m, &["-c", "-i", "tsrc", "-f", "MINI", "--quiet", "/out/s"]);
        assert_eq!((o.status, marker(&m, "/out/s")), (0, Some(want)), "after removing {gone:?}: {}", text(&o.err));
    }
    // The entries in order, each with both of its places before the next entry.
    m.files.insert(b"/e1/locales/tsrc".to_vec(), marked('B'));
    m.files.insert(b"/e0/tsrc".to_vec(), marked('E'));
    let o = go(&mut m, &["-c", "-i", "tsrc", "-f", "MINI", "--quiet", "/out/e"]);
    assert_eq!((o.status, marker(&m, "/out/e")), (0, Some(b'E')), "{}", text(&o.err));
    // copy names search the same way.
    m.files.insert(b"/e1/locales/cp".to_vec(), b"LC_NUMERIC\ncopy \"tsrc\"\nEND LC_NUMERIC\n".to_vec());
    let o = go(&mut m, &["-c", "-i", "cp", "-f", "MINI", "--quiet", "/out/c"]);
    assert_eq!((o.status, marker(&m, "/out/c")), (0, Some(b'E')), "{}", text(&o.err));
    // A relative name with '/': I18NPATH, not the default directory.
    m.files.insert(b"/usr/share/i18n/locales/sub/x".to_vec(), marked('F'));
    let o = go(&mut m, &["-c", "-i", "sub/x", "-f", "MINI", "--quiet", "/out/r"]);
    assert_eq!((o.status, marker(&m, "/out/r")), (4, None), "{}", text(&o.err));
    m.files.insert(b"/e1/locales/sub/x".to_vec(), marked('G'));
    let o = go(&mut m, &["-c", "-i", "sub/x", "-f", "MINI", "--quiet", "/out/r"]);
    assert_eq!((o.status, marker(&m, "/out/r")), (0, Some(b'G')), "{}", text(&o.err));
    // Nothing anywhere: the missing-source failure.
    let o = go(&mut m, &["-c", "-i", "nosuch", "-f", "MINI", "/out/n"]);
    assert_eq!((o.status, text(&o.err)), (4, "[error] cannot open locale definition file `nosuch': No such file or directory\n".to_string()));
}

/// s31_8_i_4: a `-f` name without `/` is tried from the working directory (not as `.gz`), then in each `I18NPATH`
/// entry's `charmaps/` and the entry itself, `<name>.gz` after `<name>`; a name with `/` is a path only.
#[test]
fn charmap_search_order() {
    let named = |n: &str| String::from_utf8(ascii_charmap()).unwrap().replace("<code_set_name> MINI", &format!("<code_set_name> {n}")).into_bytes();
    let codeset = |m: &Mem, dir: &str| -> Option<String> {
        let f = m.files.get(format!("{dir}/LC_CTYPE").as_bytes())?;
        ["CMCWD", "CMSUB", "CMGZ", "CMROOT"].iter().find(|n| f.windows(n.len()).any(|w| w == n.as_bytes())).map(|n| n.to_string())
    };
    let mut m = host();
    m.i18npath = Some(b"/e1".to_vec());
    m.files.insert(b"tcm".to_vec(), named("CMCWD"));
    m.files.insert(b"/e1/charmaps/tcm.gz".to_vec(), gzip_stored(&named("CMGZ")));
    m.files.insert(b"/e1/tcm".to_vec(), named("CMROOT"));
    let o = go(&mut m, &["-i", "mini", "-f", "tcm", "--quiet", "/out/1"]);
    assert_eq!((o.status, codeset(&m, "/out/1")), (0, Some("CMCWD".into())), "{}", text(&o.err));
    m.files.remove(&b"tcm"[..]);
    m.files.insert(b"tcm.gz".to_vec(), gzip_stored(&named("CMCWD")));
    let o = go(&mut m, &["-i", "mini", "-f", "tcm", "--quiet", "/out/2"]);
    assert_eq!((o.status, codeset(&m, "/out/2")), (0, Some("CMGZ".into())), "{}", text(&o.err));
    m.files.insert(b"/e1/charmaps/tcm".to_vec(), named("CMSUB"));
    let o = go(&mut m, &["-i", "mini", "-f", "tcm", "--quiet", "/out/3"]);
    assert_eq!((o.status, codeset(&m, "/out/3")), (0, Some("CMSUB".into())), "{}", text(&o.err));
    m.files.remove(&b"/e1/charmaps/tcm"[..]);
    m.files.remove(&b"/e1/charmaps/tcm.gz"[..]);
    let o = go(&mut m, &["-i", "mini", "-f", "tcm", "--quiet", "/out/4"]);
    assert_eq!((o.status, codeset(&m, "/out/4")), (0, Some("CMROOT".into())), "{}", text(&o.err));
    m.files.insert(b"/e1/charmaps/sub/tcm".to_vec(), named("CMSUB"));
    let o = go(&mut m, &["-i", "mini", "-f", "sub/tcm", "--quiet", "/out/5"]);
    assert_eq!(o.status, 4, "a charmap name with '/' is a path: {}", text(&o.err));
}

/// The portable character set names (XBD 6.1) this test uses, with their code points.
const POSIX_NAMES: &[(&str, u32)] = &[
    ("space", 0x20), ("percent-sign", 0x25), ("comma", 0x2c), ("hyphen", 0x2d), ("period", 0x2e), ("slash", 0x2f),
    ("zero", 0x30), ("one", 0x31), ("two", 0x32), ("three", 0x33), ("four", 0x34), ("five", 0x35), ("six", 0x36),
    ("seven", 0x37), ("eight", 0x38), ("nine", 0x39), ("colon", 0x3a), ("underscore", 0x5f),
];

fn posix_name(cp: u32) -> Option<String> {
    if cp < 0x80 && (cp as u8).is_ascii_alphabetic() {
        return Some(char::from(cp as u8).to_string());
    }
    POSIX_NAMES.iter().find(|p| p.1 == cp).map(|p| p.0.to_string())
}

/// A source over the names `n` gives each code point, and a string naming `<e-acute>` the same way.
fn named_source(n: &dyn Fn(u32) -> String) -> Vec<u8> {
    let list = |r: core::ops::RangeInclusive<u32>| r.map(|c| format!("<{}>", n(c))).collect::<Vec<_>>().join(";");
    let pairs = (0x61..=0x7a).map(|c| format!("(<{}>,<{}>)", n(c), n(c - 0x20))).collect::<Vec<_>>().join(";");
    format!(
        "LC_CTYPE\nupper {}\nlower {}\nalpha <{}>..<{}>;<{}>..<{}>\ndigit {}\nspace <{}>\ntoupper {pairs}\nEND LC_CTYPE\n\
LC_NUMERIC\ndecimal_point \"<{}>\"\nthousands_sep \"<{}>\"\ngrouping 3\nEND LC_NUMERIC\n\
LC_TIME\nd_fmt \"<{}>d<{}><{}>m<e-acute>\"\nEND LC_TIME\n",
        list(0x41..=0x5a), list(0x61..=0x7a), n(0x41), n(0x5a), n(0x61), n(0x7a), list(0x30..=0x39), n(0x20),
        n(0x2c), n(0x2e), n(0x25), n(0x2f), n(0x25)
    )
    .into_bytes()
}

/// Compile `source` over `charmap` (both by path) with `-c`; the outcome and every file written.
fn compile_pair(charmap: &[u8], source: &[u8]) -> (u8, String, BTreeMap<Vec<u8>, Vec<u8>>) {
    let mut m = Mem::default();
    m.files.insert(b"/in/cm".to_vec(), charmap.to_vec());
    m.files.insert(b"/in/src".to_vec(), source.to_vec());
    let o = go(&mut m, &["-c", "-i", "/in/src", "-f", "/in/cm", "/out/p"]);
    m.files.retain(|k, _| k.starts_with(b"/out/"));
    (o.status, text(&o.err), m.files)
}

/// s31_8_i_5: a charmap may name characters by portable character set names (XBD 6.1) without `<Uxxxx>`, and by
/// other names with the `<Uxxxx>` after the bytes; a source over those names (in lists, ranges, map pairs and
/// strings) compiles to the same files as the same source over `<Uxxxx>` names.
#[test]
fn posix_symbolic_names() {
    let mut ucs = String::from("<code_set_name> NAMES\n<mb_cur_max> 1\n<mb_cur_min> 1\nCHARMAP\n");
    let mut posix = ucs.clone();
    for c in 0..128u32 {
        ucs.push_str(&format!("<U{c:04X}> /x{c:02x}\n"));
        match posix_name(c) {
            Some(n) => posix.push_str(&format!("<{n}> /x{c:02x}\n")),
            None => posix.push_str(&format!("<U{c:04X}> /x{c:02x}\n")),
        }
    }
    ucs.push_str("<U00E9> /xe9\nEND CHARMAP\n");
    posix.push_str("<e-acute> /xe9 <U00E9> LATIN SMALL LETTER E WITH ACUTE\nEND CHARMAP\n");
    let by_ucs = named_source(&|c| format!("U{c:04X}"));
    let by_ucs = String::from_utf8(by_ucs).unwrap().replace("<e-acute>", "<U00E9>").into_bytes();
    let by_posix = named_source(&|c| posix_name(c).unwrap_or_else(|| format!("U{c:04X}")));
    let want = compile_pair(ucs.as_bytes(), &by_ucs);
    assert!(want.0 <= 1 && want.2.contains_key(&b"/out/p/LC_CTYPE"[..]), "{}", want.1);
    let got = compile_pair(posix.as_bytes(), &by_posix);
    assert_eq!((got.0, &got.1), (want.0, &want.1));
    assert!(got.2 == want.2, "the files differ from the <Uxxxx> compile");
    // A portable-names-only charmap is a charmap (no "no mappings" refusal).
    let only: String = posix.lines().filter(|l| !l.starts_with("<U")).map(|l| format!("{l}\n")).collect();
    assert_eq!(compile_pair(only.as_bytes(), &by_posix).0, want.0);
}

/// s31_8_i_5: a name the charmap does not define, in a class list, a range end, a map pair or as a collation order
/// line's item, is skipped: the
/// files and the status are those of the source without it.
#[test]
fn undefined_names_are_skipped() {
    let cm = ascii_charmap();
    let collate = |extra: &str| format!("LC_COLLATE\norder_start forward\n<U0041>\n{extra}<U0061>\nUNDEFINED\norder_end\nEND LC_COLLATE\n");
    let base = "LC_CTYPE\nupper <U0041>\nlower <U0061>\ntoupper (<U0061>,<U0041>)\nEND LC_CTYPE\n".to_string() + &collate("");
    let with = "LC_CTYPE\nupper <U0041>;<no-such>\nlower <no-such>;<U0061>;<U0062>..<no-such>\ntoupper (<U0061>,<U0041>);(<no-such>,<U0042>)\nEND LC_CTYPE\n".to_string()
        + &collate("<no-such> <no-such>\n");
    let want = compile_pair(&cm, base.as_bytes());
    let got = compile_pair(&cm, with.as_bytes());
    assert_eq!((got.0, &got.1), (want.0, &want.1));
    assert!(got.2 == want.2, "an undefined name changed the files");
}

/// s31_8_f_3: an `LC_COLLATE` without `order_start` (empty, or `collating-symbol` lines only) is refused at its END
/// line, with or without -c: status 1, the output directory made, no category file written (a file a setlocale would
/// refuse). `order_start` and `order_end` alone are a collation and compile.
#[test]
fn empty_collate_is_refused() {
    for (body, end) in [("", 13), ("collating-symbol <x>\n", 14)] {
        let src = [MINI, format!("LC_COLLATE\n{body}END LC_COLLATE\n").as_bytes()].concat();
        for force in [false, true] {
            let mut m = host();
            m.files.insert(b"/usr/share/i18n/locales/ec".to_vec(), src.clone());
            let args = ["-i", "ec", "-f", "MINI", "--quiet", "-c", "/out/e"];
            let args: Vec<&str> = args.into_iter().filter(|&a| force || a != "-c").collect();
            let o = go(&mut m, &args);
            let err = text(&o.err);
            assert_eq!(o.status, 1, "-c {force}: {err}");
            assert!(err.contains(&format!("ec:{end}: LC_COLLATE: empty category description not allowed\n")), "-c {force}: {err}");
            assert!(err.ends_with("[error] too many errors; giving up\n"), "-c {force}: {err}");
            assert!(m.dirs.iter().any(|d| d == b"/out/e"), "-c {force}: the output directory is made");
            assert!(!m.files.keys().any(|k| k.starts_with(b"/out/")), "-c {force}: a category file was written");
        }
    }
    let (_, err, files) = compile_pair(&ascii_charmap(), b"LC_COLLATE\norder_start forward\norder_end\nEND LC_COLLATE\n");
    assert!(files.contains_key(&b"/out/p/LC_COLLATE"[..]), "{err}");
}

/// How the C library reads a compiled `LC_COLLATE`: `true` for code point order (no rules).
fn code_point(file: &[u8]) -> bool {
    use niji::locale::collate::{Collate, Collation};
    let cat = niji::locale::file::Category::parse(Box::leak(file.to_vec().into_boxed_slice())).expect("a category file");
    match Collate::parse(cat) {
        Some(Collation::CodePoint) => true,
        Some(Collation::Rules(_)) => false,
        None => panic!("the C library refuses the file"),
    }
}

/// The `LC_COLLATE` compiled from `order` (the lines inside `order_start`, after `head`) with `charmap`.
fn collate_file(charmap: &[u8], head: &str, order: &str) -> Vec<u8> {
    let src = format!("LC_COLLATE\n{head}order_start forward\n{order}order_end\nEND LC_COLLATE\n");
    let (_, err, mut files) = compile_pair(charmap, src.as_bytes());
    files.remove(&b"/out/p/LC_COLLATE"[..]).unwrap_or_else(|| panic!("no LC_COLLATE: {err}\n{order}"))
}

/// s31_8_c_1 (D-31.31): an order that is every character of the charmap in code point order, each weighted by itself
/// (a `..` run or line by line), compiles without rules, in under 2 KiB, which compares as the code points do; any
/// other order keeps its tables: a character out of place (named before or after the run), a run stopping short of the
/// charmap, another weight, a contraction, `UNDEFINED`, a backward first level, or a charmap whose byte order is not
/// its code point order (a file without rules compares bytes).
#[test]
fn code_point_order_compiles_without_rules() {
    let cm = ascii_charmap();
    let run = "<U0000>\n..\n<U007F>\n";
    let lines: String = (0..128u32).map(|c| format!("<U{c:04X}>\n")).collect();
    for order in [run, lines.as_str()] {
        let f = collate_file(&cm, "", order);
        assert!(code_point(&f) && f.len() <= 2048, "{} bytes for\n{order}", f.len());
    }
    let short = "<U0000>\n..\n<U007E>\n";
    let weighted = format!("{short}<U007F> <U0041>\n");
    let after = format!("{run}<U0041>\n");
    let before = format!("<U0041>\n{run}");
    let contraction = format!("{run}<ab>\n");
    let undefined = format!("UNDEFINED\n{run}");
    for (what, head, order) in [
        ("short", "", short),
        ("weighted", "", weighted.as_str()),
        ("after", "", after.as_str()),
        ("before", "", before.as_str()),
        ("contraction", "collating-element <ab> from \"<U0061><U0062>\"\n", contraction.as_str()),
        ("undefined", "", undefined.as_str()),
    ] {
        assert!(!code_point(&collate_file(&cm, head, order)), "{what}: compiled without rules");
    }
    let backward = format!("LC_COLLATE\norder_start backward\n{run}order_end\nEND LC_COLLATE\n");
    let (_, err, files) = compile_pair(&cm, backward.as_bytes());
    assert!(!code_point(files.get(&b"/out/p/LC_COLLATE"[..]).unwrap_or_else(|| panic!("{err}"))), "backward: compiled without rules");
    let swapped = String::from_utf8(cm.clone()).unwrap().replace("<U0041> /x41", "<U0041> /x42").replacen("<U0042> /x42", "<U0042> /x41", 1);
    assert!(!code_point(&collate_file(swapped.as_bytes(), "", run)), "a charmap out of code point order: compiled without rules");
}

/// s31_8_c_1: a `..` run stays one item until a line names a character inside it; the file is then the one the same
/// order written line by line gives (the run's characters in place, the named one moved).
#[test]
fn a_run_split_by_a_later_line_compiles_as_its_lines() {
    let cm = ascii_charmap();
    let lines = |skip: u32| -> String { (0..128u32).filter(|&c| c != skip).map(|c| format!("<U{c:04X}>\n")).collect() };
    let by_run = collate_file(&cm, "", "<U0000>\n..\n<U007F>\n<U0041>\n");
    let by_lines = collate_file(&cm, "", &format!("{}<U0041>\n", lines(0x41)));
    assert!(by_run == by_lines, "moved after the run: the files differ");
    let by_run = collate_file(&cm, "", "<U0000>\n..\n<U007F>\nreorder-after <U0010>\n<U0050>\nreorder-end\n");
    let by_lines = collate_file(&cm, "", &lines(0x50).replace("<U0010>\n", "<U0010>\n<U0050>\n"));
    assert!(by_run == by_lines, "moved by reorder-after: the files differ");
    // Moved under reorder-after to an item of a backward section, a character of the run keeps its own section's rules.
    let moved = "order_end\norder_start backward\n<s>\nreorder-after <s>\n<U0041>\nreorder-end\n";
    let by_run = collate_file(&cm, "collating-symbol <s>\n", &format!("<U0000>\n..\n<U007F>\n{moved}"));
    let by_lines = collate_file(&cm, "collating-symbol <s>\n", &format!("{}{moved}", lines(u32::MAX)));
    assert!(by_run == by_lines, "moved into another section: the files differ");
}

/// A locale whose strings hold characters the ASCII charmap `MINI` lacks, with a transliteration
/// table that includes `tra`, which includes `trb` (see the test below for what each line probes).
const TR: &str = "LC_CTYPE\ntranslit_start\ninclude \"tra\";\"\"\n<U20AC> \"<U0045><U0055><U0052>\"\n\
<U2030> \"<U2031>\";\"<U0025><U0030>\"\n<U2032> \"<U2033>\"\n<U2033> \"<U0022>\"\n\
<U2034> \"\";\"<U0041><U2035>\";\"<U0042>\"\ntranslit_end\nEND LC_CTYPE\n\
LC_NUMERIC\ndecimal_point \"<U4E01>\"\nthousands_sep \"<U4E02>\"\ngrouping -1\nEND LC_NUMERIC\n\
LC_MESSAGES\nyesexpr \"<U005E>[yY<U20AC>]\"\nnoexpr \"<U005E>[nN<U4E00>]\"\nyesstr \"<U2030><U2034>\"\nnostr \"<U2032>\"\nEND LC_MESSAGES\n";
const TRA: &str = "LC_CTYPE\ntranslit_start\ninclude \"trb\";\"\"\n<U20AC> \"<U0058>\"\n<U4E01> \"<U0031>\"\ntranslit_end\nEND LC_CTYPE\n";
const TRB: &str = "LC_CTYPE\ntranslit_start\n<U4E01> \"<U0032>\"\n<U4E02> \"<U0033>\"\ntranslit_end\nEND LC_CTYPE\n";

/// Item `i` of the category file at `path` in `m`, without its NUL.
fn item(m: &Mem, path: &str, i: usize) -> String {
    let data = m.files.get(path.as_bytes()).unwrap_or_else(|| panic!("{path} written"));
    let c = niji::locale::file::Category::parse(Box::leak(data.clone().into_boxed_slice())).expect("a category file");
    text(c.item(i).split(|&b| b == 0).next().unwrap_or_default())
}

/// s31_8_g_5: a string with a character the charmap lacks takes the locale's transliteration at
/// compile time: the locale's own entry before an include's (U+20AC: `EUR`, not `tra`'s `X`), an
/// include's own entry before its include's (U+4E01: `1`, not `trb`'s `2`), an include of an
/// include (U+4E02: `3`); the first alternative the charmap encodes whole (U+2030: `%0`), an empty
/// one passed over (U+2034: `B`); no second transliteration (U+2032 to U+2033 fails). A string
/// with a character neither encoded nor transliterated is empty, one line each (under --quiet too),
/// status 1, the locale written without -c.
#[test]
fn unencodable_strings_take_the_locales_transliteration() {
    let mut m = host();
    for (name, src) in [("tr", TR), ("tra", TRA), ("trb", TRB)] {
        m.files.insert(format!("/usr/share/i18n/locales/{name}").into_bytes(), src.as_bytes().to_vec());
    }
    let o = go(&mut m, &["-i", "tr", "-f", "MINI", "--quiet", "/out/tr"]);
    let err = text(&o.err);
    assert_eq!(o.status, 1, "{err}");
    assert_eq!(err, "tr:18: LC_MESSAGES: unknown character in field `noexpr'\ntr:20: LC_MESSAGES: unknown character in field `nostr'\n");
    assert_eq!((item(&m, "/out/tr/LC_NUMERIC", 0), item(&m, "/out/tr/LC_NUMERIC", 1)), ("1".into(), "3".into()));
    let msgs: Vec<String> = (0..4).map(|i| item(&m, "/out/tr/LC_MESSAGES/SYS_LC_MESSAGES", i)).collect();
    assert_eq!(msgs, ["^[yYEUR]", "", "%0B", ""]);
}

// ---- The `locales` package's commands (niji_locale::cmds; D-31.36, D-31.37) ----

const ARCHIVE: &str = "/usr/lib/locale/locale-archive";

/// `host()` with sources `b_B`, `c_C`, `d_D` and `e_E@mod` (copies of MINI) and `/etc/locale.gen` = `list`.
fn gen_host(list: &str) -> Mem {
    let mut m = host();
    for name in ["b_B", "c_C", "d_D", "e_E@mod"] {
        m.files.insert(format!("/usr/share/i18n/locales/{name}").into_bytes(), MINI.to_vec());
    }
    m.files.insert(b"/etc/locale.gen".to_vec(), list.as_bytes().to_vec());
    m
}

fn cmd(m: &mut Mem, argv0: &str, args: &[&str]) -> Outcome {
    let args: Vec<&[u8]> = args.iter().map(|a| a.as_bytes()).collect();
    niji_locale::cmds::by_name(argv0.as_bytes(), &args, m).expect("a locales command")
}

/// The archive's names, sorted (`localedef --list-archive`).
fn archived(m: &mut Mem) -> String {
    text(&go(m, &["--list-archive"]).out)
}

/// s31_14_c_2: `locale-gen` compiles exactly the uncommented `/etc/locale.gen` lines (a `#` line, even after
/// blanks, is not selected; leading blanks and a tab separator are), each `<locale> <charset>`: localedef `-i`
/// the locale without its codeset (`e_E@mod`), `-f` the charset, stored under the locale as written (codeset
/// normalized). Progress on standard output: the locale shown with the charset as codeset; a line without a charset is a `Bad entry`
/// and is skipped; status 0. Without --keep-existing the archive is replaced whole.
#[test]
fn locale_gen_compiles_the_uncommented_lines() {
    let mut m = gen_host("# comment\n#a_A.MINI MINI\n  # c_C MINI\n\n  b_B.MINI MINI\nc_C\tMINI\nd_D\ne_E.X@mod MINI\n");
    m.files.insert(ARCHIVE.as_bytes().to_vec(), b"an old archive".to_vec());
    let o = cmd(&mut m, "/usr/sbin/locale-gen", &[]);
    assert_eq!(o.status, 0, "{}", text(&o.err));
    assert_eq!(
        text(&o.out),
        "Generating locales (this might take a while)...\n  b_B.MINI... done\n  c_C.MINI... done\n\
         error: Bad entry 'd_D '\n  e_E.MINI@mod... done\nGeneration complete.\n"
    );
    assert_eq!(archived(&mut m), "b_B.mini\nc_C\nc_C.mini\ne_E.x@mod\n");
}

/// s31_14_c_2: `--keep-existing` keeps the archive and compiles only the names it lacks (silently skipping the
/// others); a missing `/etc/locale.gen` prints nothing, exits 0 and leaves the archive; any other argument is
/// ignored.
#[test]
fn locale_gen_keep_existing_and_no_list() {
    let mut m = gen_host("b_B.MINI MINI\n");
    assert_eq!(cmd(&mut m, "locale-gen", &["--purge"]).status, 0);
    m.files.insert(b"/etc/locale.gen".to_vec(), b"b_B.MINI MINI\nc_C.MINI MINI\n".to_vec());
    let o = cmd(&mut m, "locale-gen", &["--keep-existing"]);
    assert_eq!(text(&o.out), "Generating locales (this might take a while)...\n  c_C.MINI... done\nGeneration complete.\n");
    assert_eq!(archived(&mut m), "b_B.mini\nc_C.mini\n");
    m.files.remove(b"/etc/locale.gen".as_slice());
    let o = cmd(&mut m, "locale-gen", &[]);
    assert_eq!((o.status, text(&o.out), text(&o.err)), (0, String::new(), String::new()));
    assert_eq!(archived(&mut m), "b_B.mini\nc_C.mini\n");
}

fn conf(m: &Mem, path: &str) -> String {
    text(m.files.get(path.as_bytes()).map(Vec::as_slice).unwrap_or(b"<none>"))
}

/// s31_14_c_2: `update-locale` writes `/etc/locale.conf` (a new file starts with its header line; nothing else
/// is created); `--locale-file` names another file. `NAME=value` replaces the variable's line in place, active or
/// commented, else is appended; a bare `NAME` comments it out; an empty line is written back as `#`; `export`
/// lines are not variables; `--reset` comments every active variable the arguments do not name.
#[test]
fn update_locale_edits_the_locale_file() {
    let mut m = host();
    let o = cmd(&mut m, "update-locale", &["LANG=C"]);
    assert_eq!((o.status, text(&o.err)), (0, String::new()));
    assert_eq!(conf(&m, "/etc/locale.conf"), "#  File generated by update-locale\nLANG=C\n");
    assert_eq!(m.files.len(), 3, "only /etc/locale.conf is written");
    m.files.insert(b"/x/loc".to_vec(), b"  LC_PAPER=C\n\n#LANG=POSIX\nexport LC_TIME=C\nLC_NAME=C\n".to_vec());
    let o = cmd(&mut m, "update-locale", &["--locale-file", "/x/loc", "LANG=C", "LC_PAPER", "LC_TIME=POSIX"]);
    assert_eq!((o.status, text(&o.err)), (0, String::new()));
    assert_eq!(conf(&m, "/x/loc"), "#  LC_PAPER=C\n#\nLANG=C\nexport LC_TIME=C\nLC_NAME=C\nLC_TIME=POSIX\n");
    let o = cmd(&mut m, "update-locale", &["--locale-file=/x/loc", "--reset", "LC_TIME=C"]);
    assert_eq!(o.status, 0);
    assert_eq!(conf(&m, "/x/loc"), "#  LC_PAPER=C\n#\n#LANG=C\nexport LC_TIME=C\n#LC_NAME=C\nLC_TIME=C\n");
    assert_eq!(conf(&m, "/etc/locale.conf"), "#  File generated by update-locale\nLANG=C\n");
    let o = cmd(&mut m, "update-locale", &["--bogus", "--locale-file"]);
    assert_eq!((o.status, text(&o.err)), (0, "Unknown option: bogus\nOption locale-file requires an argument\n".into()));
}

/// s31_14_c_2: a locale setting (`LANG`, `LC_*`) that is not available is refused: the settings on standard
/// error, status 255, the file unchanged, before `LANGUAGE` is looked at. Then `LANGUAGE` whose first language
/// is not `LC_ALL`'s (else `LANG`'s) is disabled with a warning on standard output, unless that locale is C or
/// POSIX. `--no-checks` skips both checks.
#[test]
fn update_locale_checks_and_language() {
    let mut m = host();
    assert_eq!(cmd(&mut m, "update-locale", &["LANG=C", "LANGUAGE=fr:en"]).status, 0);
    let o = cmd(&mut m, "update-locale", &["LC_TIME=xx_XX.UTF-8", "LANG=C.UTF-8"]);
    assert_eq!(
        (o.status, text(&o.out), text(&o.err)),
        (255, String::new(), "*** update-locale: Error: invalid locale settings:  LANG=C.UTF-8 LANGUAGE=fr:en LC_TIME=xx_XX.UTF-8\n".into())
    );
    assert_eq!(conf(&m, "/etc/locale.conf"), "#  File generated by update-locale\nLANG=C\nLANGUAGE=fr:en\n");
    let o = cmd(&mut m, "update-locale", &["--no-checks", "LANG=de_DE.UTF-8"]);
    assert_eq!((o.status, text(&o.out), text(&o.err)), (0, String::new(), String::new()));
    assert_eq!(conf(&m, "/etc/locale.conf"), "#  File generated by update-locale\nLANG=de_DE.UTF-8\nLANGUAGE=fr:en\n");
    let o = cmd(&mut m, "update-locale", &["LANG=C.UTF-8"]);
    assert_eq!(
        (o.status, text(&o.out)),
        (0, "*** update-locale: Warning: LANGUAGE (fr:en) is not compatible with LANG (C.UTF-8). Disabling it.\n".into())
    );
    assert_eq!(conf(&m, "/etc/locale.conf"), "#  File generated by update-locale\nLANG=C.UTF-8\n#LANGUAGE=fr:en\n");
    let o = cmd(&mut m, "update-locale", &["LANGUAGE=de:en", "LC_ALL=C.UTF-8"]);
    assert!(text(&o.out).contains("with LC_ALL (C.UTF-8)"), "{}", text(&o.out));
    let o = cmd(&mut m, "update-locale", &["LANGUAGE=de:en", "LC_ALL=C"]);
    assert_eq!((o.status, text(&o.out)), (0, String::new()));
    assert_eq!(conf(&m, "/etc/locale.conf"), "#  File generated by update-locale\nLANG=C.UTF-8\nLANGUAGE=de:en\nLC_ALL=C\n");
}

/// s31_14_c_2: `validlocale` accepts C, POSIX, C.UTF-8, the empty name and an archived locale (by any spelling
/// of its codeset): standard error only, status 0. Else status 1 and the `/etc/locale.gen` line to add: its
/// `SUPPORTED` line, or the charset `DEFAULTCHARSET` names, ISO-8859-1 without it. No operand: usage, status 1.
#[test]
fn validlocale_outputs() {
    let mut m = gen_host("b_B.MINI MINI\n");
    assert_eq!(cmd(&mut m, "locale-gen", &[]).status, 0);
    m.files.insert(b"/usr/share/i18n/SUPPORTED".to_vec(), b"f_F.UTF-8 UTF-8\nf_F ISO-8859-15\n".to_vec());
    for name in ["C", "POSIX", "C.UTF-8", "", "b_B.MINI", "b_B.mini"] {
        let o = cmd(&mut m, "validlocale", &[name]);
        assert_eq!((o.status, text(&o.out), text(&o.err)), (0, String::new(), format!("locale '{name}' valid and available\n")));
    }
    let o = cmd(&mut m, "validlocale", &["f_F", "ignored"]);
    assert_eq!((o.status, text(&o.out), text(&o.err)), (1, "f_F ISO-8859-15\n".into(), "locale 'f_F' not available\n".into()));
    assert_eq!(text(&cmd(&mut m, "validlocale", &["b_B"]).out), "b_B ISO-8859-1\n");
    m.defaultcharset = Some(b"UTF-8".to_vec());
    assert_eq!(text(&cmd(&mut m, "validlocale", &["g_G"]).out), "g_G UTF-8\n");
    let o = cmd(&mut m, "/usr/sbin/validlocale", &[]);
    assert_eq!((o.status, text(&o.out)), (1, "Usage: /usr/sbin/validlocale <locale>\n".into()));
    // A bare argv[0] is named by the first PATH directory holding it, as a shell names a script.
    assert_eq!(text(&cmd(&mut m, "validlocale", &[]).out), "Usage: validlocale <locale>\n");
    m.files.insert(b"/usr/sbin/validlocale".to_vec(), Vec::new());
    m.path = Some(b"/nowhere::/usr/sbin:/usr/bin".to_vec());
    assert_eq!(text(&cmd(&mut m, "validlocale", &[]).out), "Usage: /usr/sbin/validlocale <locale>\n");
    assert!(niji_locale::cmds::by_name(b"/usr/bin/localedef", &[], &mut m).is_none());
}
