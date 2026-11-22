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

/// s31_8_g_1: a NAME with `/` gets the directory form: the directory, `LC_MESSAGES/`, and one file per category
/// the source defines, each the library's compiled file; a category the source lacks is a warning and status 1
/// (s31_8_c_1).
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
    assert_eq!(m.files.len(), 4, "only the two category files are added");
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

/// s31_8_g_1: a NAME without `/` with `--no-archive` goes under the locale path (after `--prefix`); without it the
/// archive is asked for, which this localedef does not write yet: status 4, nothing written.
#[test]
fn name_without_slash() {
    let mut m = host();
    let o = go(&mut m, &["-i", "mini", "-f", "MINI", "--no-archive", "--prefix=/root", "--quiet", "mini.MINI"]);
    assert_eq!((o.status, text(&o.err)), (0, String::new()));
    assert!(m.files.contains_key(&b"/root/usr/lib/locale/mini.MINI/LC_NUMERIC"[..]));
    let mut m = host();
    let o = go(&mut m, &["-i", "mini", "-f", "MINI", "mini.MINI"]);
    assert_eq!(o.status, 4);
    assert!(text(&o.err).starts_with("[error] "), "{}", text(&o.err));
    assert!(m.dirs.is_empty());
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
