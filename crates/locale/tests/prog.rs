//! The `localedef` command (`niji_locale::prog`) over an in-memory file system: the options, the
//! input search paths, the directory form and the messages and statuses (spec §31.8).
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
/// with `/` skips the default directory; none readable: status 4.
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
        assert_eq!((o.status, marker(&m, "/out/s")), (1, Some(want)), "after removing {gone:?}: {}", text(&o.err));
    }
    // The entries in order, each with both of its places before the next entry.
    m.files.insert(b"/e1/locales/tsrc".to_vec(), marked('B'));
    m.files.insert(b"/e0/tsrc".to_vec(), marked('E'));
    let o = go(&mut m, &["-c", "-i", "tsrc", "-f", "MINI", "--quiet", "/out/e"]);
    assert_eq!((o.status, marker(&m, "/out/e")), (1, Some(b'E')), "{}", text(&o.err));
    // copy names search the same way.
    m.files.insert(b"/e1/locales/cp".to_vec(), b"LC_NUMERIC\ncopy \"tsrc\"\nEND LC_NUMERIC\n".to_vec());
    let o = go(&mut m, &["-c", "-i", "cp", "-f", "MINI", "--quiet", "/out/c"]);
    assert_eq!((o.status, marker(&m, "/out/c")), (1, Some(b'E')), "{}", text(&o.err));
    // A relative name with '/': I18NPATH, not the default directory.
    m.files.insert(b"/usr/share/i18n/locales/sub/x".to_vec(), marked('F'));
    let o = go(&mut m, &["-c", "-i", "sub/x", "-f", "MINI", "--quiet", "/out/r"]);
    assert_eq!((o.status, marker(&m, "/out/r")), (4, None), "{}", text(&o.err));
    m.files.insert(b"/e1/locales/sub/x".to_vec(), marked('G'));
    let o = go(&mut m, &["-c", "-i", "sub/x", "-f", "MINI", "--quiet", "/out/r"]);
    assert_eq!((o.status, marker(&m, "/out/r")), (1, Some(b'G')), "{}", text(&o.err));
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
    if (b'A'..=b'Z').contains(&(cp as u8)) && cp < 0x80 || (b'a'..=b'z').contains(&(cp as u8)) && cp < 0x80 {
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

/// s31_8_i_5: a name the charmap does not define, in a class list, a range end or a map pair, is skipped: the
/// files and the status are those of the source without it.
#[test]
fn undefined_names_are_skipped() {
    let cm = ascii_charmap();
    let base = "LC_CTYPE\nupper <U0041>\nlower <U0061>\ntoupper (<U0061>,<U0041>)\nEND LC_CTYPE\n";
    let with = "LC_CTYPE\nupper <U0041>;<no-such>\nlower <no-such>;<U0061>;<U0062>..<no-such>\ntoupper (<U0061>,<U0041>);(<no-such>,<U0042>)\nEND LC_CTYPE\n";
    let want = compile_pair(&cm, base.as_bytes());
    let got = compile_pair(&cm, with.as_bytes());
    assert_eq!((got.0, &got.1), (want.0, &want.1));
    assert!(got.2 == want.2, "an undefined name changed the files");
}
