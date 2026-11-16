//! Constructor and destructor order (phase 14 DL-11 `DL-INIT-ORDER`, plan/ph14/design.md §9,
//! D-27.19).
//!
//! Trace: s1_1_c_5 (ELF constructors, `DT_PREINIT_ARRAY`, `DT_INIT`, `DT_INIT_ARRAY`, in dependency
//! order), s1_1_c_6 (destructors registered to run at normal exit), s27_1_c_1 (the constructors of a
//! set opened by `dlopen`: the order rule for its root).
//!
//! Two layers: (1) [`niji_ld::init`] over the dependency graphs of the order corpus, the expected
//! orders being the reference loader's (tools/probe/dl/order.sh, and DL-0b's `== order` record in
//! spec/behaviour/dynlink.glibc.txt for `dlopen` and `exit` with an object opened later); (2) the
//! corpus itself (`-nostdlib` objects printing what runs) under the built `ld.so`, compared line for
//! line with the reference's record below and, when the system loader runs the corpus, with the
//! reference live. The reference's termination function needs its C library, so the exit lines of
//! the record come from the same programs linked with the C library (`*_libc`).
// ARCH(x86_64): the corpus is built by the host toolchain with x86-64 inline assembly.
#![cfg(target_arch = "x86_64")]
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

use niji_ld::arch::{Arch, Native};
use niji_ld::init::{self, OrderError, OBJECTS_MAX};

fn init(n: usize, edges: &[(u8, u8)]) -> Vec<u8> {
    let mut out = [0u8; OBJECTS_MAX];
    let len = init::init_order(n, edges, &mut out).unwrap();
    out[..len].to_vec()
}

fn fini(n: usize, edges: &[(u8, u8)]) -> Vec<u8> {
    let mut out = [0u8; OBJECTS_MAX];
    let len = init::fini_order(n, edges, &mut out).unwrap();
    out[..len].to_vec()
}

/// s1_1_c_5, s1_1_c_6: the DL-0b graph (main -> A, D; A -> B, C; D -> C; load order main A D B C) initialises
/// C, B, D, A, then the program, and finalises in reverse.
#[test]
fn init_order_dependencies_first() {
    let e = [(0, 1), (0, 2), (1, 3), (1, 4), (2, 4)];
    assert_eq!(init(5, &e), [4, 3, 2, 1, 0], "dependencies are initialised before their dependents");
    assert_eq!(fini(5, &e), [0, 1, 2, 3, 4], "the finalisation order is the initialisation order reversed");
}

/// s1_1_c_5, s1_1_c_6: ties and a dependency loaded through a later sibling, as the reference orders them
/// (tools/probe/dl/order.sh `p_tie`, `p_sib`).
#[test]
fn init_order_ties_and_siblings() {
    // main -> R, Q, P; P -> R (load order main R Q P): R, P, Q.
    let tie = [(0, 1), (0, 2), (0, 3), (3, 1)];
    assert_eq!(init(4, &tie), [1, 3, 2, 0]);
    assert_eq!(fini(4, &tie), [0, 2, 3, 1]);
    // main -> L, K; L -> N; K -> M; N -> K (load order main L K N M): M, K, N, L.
    let sib = [(0, 1), (0, 2), (1, 3), (2, 4), (3, 2)];
    assert_eq!(init(5, &sib), [4, 2, 3, 1, 0]);
    assert_eq!(fini(5, &sib), [0, 1, 3, 2, 4]);
}

/// s1_1_c_5, s27_1_c_1: cycles as the reference orders them: at start (`p_cycle`, `p_cycle3`) and for a set
/// opened by `dlopen` (DL-0b: `dlopen("libH")`, H <-> I: I then H).
#[test]
fn init_order_cycles() {
    // main -> H; H -> I; I -> H: H, I.
    assert_eq!(init(3, &[(0, 1), (1, 2), (2, 1)]), [1, 2, 0]);
    // main -> X; X -> Y; Y -> Z, W; Z -> X (load order main X Y Z W): W, Y, X, Z.
    let c3 = [(0, 1), (1, 2), (2, 3), (2, 4), (3, 1)];
    assert_eq!(init(5, &c3), [4, 2, 1, 3, 0]);
    assert_eq!(fini(5, &c3), [0, 3, 1, 2, 4]);
    // The root of a dlopen set is never entered from another object: H (root) <-> I gives I, H.
    assert_eq!(init(2, &[(0, 1), (1, 0)]), [1, 0]);
}

/// s1_1_c_6: at exit an object opened later is placed by its dependencies (DL-0b `exit`: main A D B
/// E C F with E -> C, F opened after start).
#[test]
fn init_order_exit_with_object_opened_later() {
    let e = [(0, 1), (0, 2), (1, 3), (1, 4), (2, 4), (5, 4), (5, 6)];
    assert_eq!(fini(7, &e), [0, 1, 2, 3, 5, 4, 6]);
}

/// no spec clause: limits are refused, not walked past.
#[test]
fn init_order_refuses_bad_input() {
    let mut out = [0u8; OBJECTS_MAX];
    assert_eq!(init::init_order(OBJECTS_MAX + 1, &[], &mut out), Err(OrderError::TooMany));
    assert_eq!(init::init_order(2, &[(0, 2)], &mut out), Err(OrderError::BadEdge));
    assert_eq!(init::init_order(0, &[], &mut out), Ok(0));
}

/// s1_1_c_5, s1_1_c_6: one object's calls: `DT_INIT` before `DT_INIT_ARRAY` (in order); `DT_FINI_ARRAY` in
/// reverse before `DT_FINI`; 0 and all-ones entries left out (D-27.19).
#[test]
fn init_order_calls_of_one_object() {
    let init: Vec<u64> = init::init_calls(0x10, &[0x20, 0, 0x30, u64::MAX]).collect();
    assert_eq!(init, [0x10, 0x20, 0x30], "DT_INIT runs before DT_INIT_ARRAY, whose entries run in order");
    let fini: Vec<u64> = init::fini_calls(0x10, &[0x20, 0x30]).collect();
    assert_eq!(fini, [0x30, 0x20, 0x10], "DT_FINI_ARRAY runs in reverse, then DT_FINI");
    assert_eq!(init::init_calls(0, &[]).count(), 0);
}

// ------------------------------------------------------------------ (2) the corpus under ld.so

fn target() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from)
}

fn tools_ok() -> bool {
    for (tool, arg) in [("cc", "--version"), ("perl", "-v"), ("bash", "--version")] {
        if !Command::new(tool).arg(arg).output().is_ok_and(|o| o.status.success()) {
            eprintln!("SKIP: no {tool} to build the order corpus");
            return false;
        }
    }
    true
}

/// The reference loader's record (tools/probe/dl/order.sh on the x86-64 reference host,
/// 2026-11-10): for each program, the start lines (the program's preinit array, then the shared
/// objects' initialisation; the program's own constructors are its C library's, so they do not
/// appear), and the exit lines of the same program linked with the C library.
const RECORD: &[(&str, &str, &str)] = &[
    (
        "p_order",
        "preinit_array main\nctor C\nctor B\nDT_INIT D\nctor D\nctor200 A\nctor300 A\nctor A\n",
        "fini_array main\ndtor main\ndtor A\ndtor300 A\ndtor200 A\ndtor D\nDT_FINI D\ndtor B\ndtor C\n",
    ),
    ("p_cycle", "preinit_array main\nctor H\nctor I\n", "fini_array main\ndtor main\ndtor I\ndtor H\n"),
    ("p_tie", "preinit_array main\nctor R\nctor P\nctor Q\n", "fini_array main\ndtor main\ndtor Q\ndtor P\ndtor R\n"),
    ("p_sib", "preinit_array main\nctor M\nctor K\nctor N\nctor L\n", "fini_array main\ndtor main\ndtor L\ndtor N\ndtor K\ndtor M\n"),
    ("p_cycle3", "preinit_array main\nctor W\nctor Y\nctor X\nctor Z\n", "fini_array main\ndtor main\ndtor Z\ndtor X\ndtor Y\ndtor W\n"),
    // libS's DT_INIT_ARRAY retagged DT_PREINIT_ARRAY: a shared object's preinit array is not run.
    ("p_spre", "preinit_array main\n", "fini_array main\ndtor main\ndtor S\n"),
];

/// The x86-64 psABI's program interpreter: the loader the corpus programs name in `PT_INTERP`.
const REFERENCE: &str = "/lib64/ld-linux-x86-64.so.2";

/// Run the corpus script; its output split into `== name` sections.
fn corpus(build: &Path, ldso: Option<&Path>) -> Vec<(String, String)> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/probe/dl/order.sh");
    let mut c = Command::new("bash");
    c.arg(&script).arg(build);
    if let Some(l) = ldso {
        c.arg(l);
    }
    let o = c.output().unwrap();
    assert!(o.status.success(), "order.sh: {}", String::from_utf8_lossy(&o.stderr));
    let text = String::from_utf8(o.stdout).unwrap();
    let mut out: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        if let Some(name) = line.strip_prefix("== ") {
            out.push((name.to_owned(), String::new()));
        } else if let Some((_, body)) = out.last_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    out
}

fn section<'a>(secs: &'a [(String, String)], name: &str) -> &'a str {
    secs.iter().find(|(n, _)| n == name).map_or_else(|| panic!("order.sh printed no section {name}"), |(_, b)| b.as_str())
}

/// s1_1_c_5, s1_1_c_6: the order corpus under the built `ld.so` prints, line for line,
/// the reference's start order, then `main`, then (through the termination function it got in
/// `%rdx`) the reference's exit order.
#[test]
fn init_order_corpus_matches_reference() {
    if !tools_ok() {
        return;
    }
    let ld = target().join("niji").join(Native::INTERP);
    if !ld.exists() {
        eprintln!("SKIP: {} not built (run cargo xtask so)", ld.display());
        return;
    }
    let d = target().join(format!("ld-init-order-{}", std::process::id()));
    let ours = corpus(&d, Some(&ld));
    for (name, start, exit) in RECORD {
        let want = format!("{start}main\ncall fini\n{exit}   rc=0\n");
        assert_eq!(section(&ours, name), want, "{name} under libniji's ld.so (left) differs from the reference's order (right)");
    }
    // The reference live: the same build started through its PT_INTERP (start lines), and the C
    // library variants (exit lines).
    if Path::new(REFERENCE).exists() {
        let theirs = corpus(&d, None);
        for (name, start, exit) in RECORD {
            let s = section(&theirs, name);
            assert!(s.starts_with(&format!("{start}main\n")), "{name}: the reference loader now prints\n{s}");
            let s = section(&theirs, &format!("{name}_libc"));
            assert!(s.ends_with(&format!("main\n{exit}   rc=0\n")), "{name}_libc: the reference now prints\n{s}");
        }
    } else {
        eprintln!("SKIP (live reference part): no {REFERENCE}");
    }
    std::fs::remove_dir_all(&d).ok();
}

/// [`init::exit_order`] over named units: `seq` in sequence order, `before` pairs by name, `inits`
/// the initialisation order (the rank of a unit missing from it is 0).
fn exit(seq: &[&'static str], before: &[(&str, &str)], inits: &[&str]) -> Vec<&'static str> {
    let at = |s: &str| seq.iter().position(|&x| x == s).unwrap();
    let mut succ: Vec<Vec<usize>> = vec![Vec::new(); seq.len()];
    for &(a, b) in before {
        succ[at(a)].push(at(b));
    }
    let rank: Vec<u32> = seq.iter().map(|s| inits.iter().position(|x| x == s).map_or(0, |p| u32::try_from(p + 1).unwrap())).collect();
    let mut out = vec![0u16; seq.len()];
    let len = init::exit_order(seq.len(), |v, e| succ[v].get(e).copied(), &rank, &mut out).unwrap();
    out[..len].iter().map(|&v| seq[usize::from(v)]).collect()
}

/// s1_1_c_6, s27_1_c_1 (probe tools/probe/dl/exitorder E1-E30 and DL-0b's `== order` exit record;
/// story DL-EXIT-ORDER-PROBE): at exit the program, the start-time objects and the `dlopen`ed
/// objects run as one sequence, the earliest loaded first, each object after every object that
/// needs it; a cycle runs as one group, the last initialised first. `libc` stands for the C library
/// every probe object needs.
#[test]
fn exit_order_load_order_dependents_first() {
    let lc = |xs: &[&'static str]| xs.iter().map(|&x| (x, "libc")).collect::<Vec<_>>();
    // `== order` exit: main needs A, D; A needs B, C; D needs C; E (opened) needs C, F.
    let mut b = vec![("main", "A"), ("A", "D"), ("D", "B"), ("B", "C"), ("C", "libc"), ("E", "C"), ("E", "F")];
    b.extend(lc(&["E", "F"]));
    assert_eq!(exit(&["main", "A", "D", "B", "C", "libc", "E", "F"], &b, &[]), ["main", "A", "D", "B", "E", "C", "F", "libc"]);
    // E1-E3, E8, E9: independent objects (kept open, NODELETE or both) run the earliest loaded first.
    let mut b = vec![("main", "libc")];
    b.extend(lc(&["A", "B", "C"]));
    assert_eq!(exit(&["main", "libc", "A", "B", "C"], &b, &[]), ["main", "A", "B", "C", "libc"]);
    // E14, E17: a dependency loaded first runs after the object that needs it.
    let mut b = vec![("main", "libc"), ("D", "C")];
    b.extend(lc(&["C", "D"]));
    assert_eq!(exit(&["main", "libc", "C", "D"], &b, &[]), ["main", "D", "C", "libc"]);
    let mut b = vec![("main", "libc"), ("D", "C")];
    b.extend(lc(&["C", "A", "D"]));
    assert_eq!(exit(&["main", "libc", "C", "A", "D"], &b, &[]), ["main", "A", "D", "C", "libc"]);
    // E18: H2 needs D needs C, loaded D, C, H2.
    let mut b = vec![("main", "libc"), ("D", "C"), ("H2", "D")];
    b.extend(lc(&["D", "C", "H2"]));
    assert_eq!(exit(&["main", "libc", "D", "C", "H2"], &b, &[]), ["main", "H2", "D", "C", "libc"]);
    // E24, E25: L is a start-time object; M (opened) needs it, A and B do not.
    let mut b = vec![("main", "L"), ("L", "libc"), ("M", "L")];
    b.extend(lc(&["A", "M", "B"]));
    assert_eq!(exit(&["main", "L", "libc", "A", "M", "B"], &b, &[]), ["main", "A", "M", "L", "B", "libc"]);
    let mut b = vec![("main", "L"), ("L", "libc")];
    b.extend(lc(&["A"]));
    assert_eq!(exit(&["main", "L", "libc", "A"], &b, &[]), ["main", "L", "A", "libc"]);
    // E20: the cycle H <-> I (initialised I, H) runs as one group between A and B, H first.
    let mut b = vec![("main", "libc"), ("H", "I"), ("I", "H")];
    b.extend(lc(&["A", "H", "I", "B"]));
    assert_eq!(exit(&["main", "libc", "A", "H", "I", "B"], &b, &["A", "I", "H", "B"]), ["main", "A", "H", "I", "B", "libc"]);
    // E29, E30: X -> Y -> Z -> X (initialised Z, Y, X) and K needing Y: K, then X, Y, Z.
    let mut b = vec![("main", "libc"), ("X", "Y"), ("Y", "Z"), ("Z", "X"), ("K", "Y")];
    b.extend(lc(&["X", "Y", "Z", "K"]));
    let inits = ["Z", "Y", "X", "K"];
    assert_eq!(exit(&["main", "libc", "X", "Y", "Z", "K"], &b, &inits), ["main", "K", "X", "Y", "Z", "libc"]);
    assert_eq!(exit(&["main", "libc", "K", "Y", "Z", "X"], &b, &inits), ["main", "K", "X", "Y", "Z", "libc"]);
    // A successor outside the units, or more units than the table.
    let mut out = [0u16; 2];
    assert_eq!(init::exit_order(2, |v, e| (v == 0 && e == 0).then_some(2), &[0, 0], &mut out), Err(OrderError::BadEdge));
    let mut big = vec![0u16; init::EXIT_MAX + 1];
    assert_eq!(init::exit_order(init::EXIT_MAX + 1, |_, _| None, &vec![0; init::EXIT_MAX + 1], &mut big), Err(OrderError::TooMany));
}
