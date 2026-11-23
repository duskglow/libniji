//! printf allocates nowhere the reference does not (PRINTF-NO-HEAP; spec §12.6, `s12_6_f_2`: a
//! formatted-output call fails with `ENOMEM` only where it needs memory).
//!
//! Probe record (glibc 2.39, a C program run with a malloc replacement that counts calls, then
//! refuses every one; built with `-fno-builtin -fno-printf-return-value`; no glibc source was
//! consulted). No allocation, and the same result when every allocation is refused, for: plain
//! conversions; `%f` of 1e308 and -1e308, `%Lf` of 1e300 and 1e4000, `%.400e`, `%.10000f`,
//! `%.1100f` of 1e-300, `%'f`, `%0400f`, `%-400e`, `%+.300f`; integer precisions and widths up to
//! 100000 (`%.500d`, `%.100000d`, `%.100000x`, `%#.600o`, `%+0600.500d`, `%100000d`, `%-100000d`,
//! `%0100000d`, `%*d`, `%.*d`, `%'.500d`); `%100000s`, a 100000-byte `%s`, `%.50000s`, `%ls` (a
//! 10000-character one too), `%lc`, `%300c`, `%p`, `%m`, `%.300a`, `%.5000a`; up to 12 positional
//! arguments; any number of sequential conversions (100 tried, of `%d`, `%'d`, `%+.3d`, and 50
//! of `%*d`); and the same in `swprintf` (100 sequential `%d` and `%m` too), which fails with
//! `E2BIG` (7) whenever the result does not fit, at any size. The reference allocates (and fails with
//! `ENOMEM` under refusal) for a 100000-digit float precision (`%.100000f`, `%.100000e`,
//! `%#.100000g`) and from 16 positional arguments on: those rows are not held to zero here.
//!
//! Each row runs once with allocation allowed (the reference text) and once with every
//! allocation of the calling thread refused (`failinject`): it must make no allocator call and
//! give the same return value, `errno` and text.
#![no_builtins] // calls to libc names reach libniji's export, not LLVM's folding (OPAQUE-AUDIT)
#![allow(clippy::arithmetic_side_effects, clippy::unwrap_used, clippy::indexing_slicing, clippy::as_conversions, clippy::cast_possible_truncation, clippy::cast_possible_wrap, missing_docs)]

use niji_abi::ctypes::{c_char, c_void, wchar_t};
use niji_abi::layout::va_list;
use niji_boundary::errno;
use niji_boundary::posix::s12_stdio as x;
use niji_boundary::selftest::failinject::{self, FailAlloc};

#[global_allocator]
static ALLOC: FailAlloc = FailAlloc;

/// One variadic argument.
#[derive(Clone, Copy)]
enum A {
    I(u64),
    D(f64),
    /// An 80-bit `long double`: the significand and the sign-and-exponent word.
    L(u64, u16),
}

/// `2^e` as an 80-bit `long double`.
fn pow2l(e: u16) -> A {
    A::L(1 << 63, 16383 + e)
}

/// A `va_list` whose arguments all sit in its overflow area (16-byte aligned for `long double`).
struct Va {
    /// The memory `ap` points into, kept alive with it.
    _area: Vec<u128>,
    ap: va_list,
}

fn va(args: &[A]) -> Va {
    let mut words: Vec<u64> = Vec::new();
    for a in args {
        match *a {
            A::I(v) => words.push(v),
            A::D(d) => words.push(d.to_bits()),
            A::L(m, se) => {
                if words.len() % 2 == 1 {
                    words.push(0);
                }
                words.push(m);
                words.push(u64::from(se));
            }
        }
    }
    words.push(0);
    let area: Vec<u128> = words.chunks(2).map(|c| u128::from(c[0]) | (u128::from(*c.get(1).unwrap_or(&0)) << 64)).collect();
    let ap = va_list { gp_offset: 48, fp_offset: 304, overflow_arg_area: area.as_ptr() as usize, reg_save_area: 0 };
    Va { _area: area, ap }
}

/// What one call gave: return value, `errno` (0 on success), the text's hash.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Res {
    rc: i32,
    err: i32,
    hash: u64,
}

fn fnv(bytes: impl Iterator<Item = u8>) -> u64 {
    bytes.fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

const BIG: usize = 1 << 20;

/// The output of one call through `v` (built beforehand: the call may run with allocation
/// refused), as many units as `rc` says (the terminator excluded). No allocation here.
fn narrow(buf: &mut [u8], max: usize, fmt: &[u8], v: &mut Va) -> Res {
    errno::set_raw(0);
    let rc = x::vsnprintf(buf.as_mut_ptr().cast::<c_char>(), max, fmt.as_ptr().cast::<c_char>(), (&raw mut v.ap).cast::<c_void>());
    let err = if rc < 0 { errno::get() } else { 0 };
    let n = usize::try_from(rc).unwrap_or(0).min(max.saturating_sub(1));
    Res { rc, err, hash: fnv(buf[..n].iter().copied()) }
}

fn wide(buf: &mut [wchar_t], max: usize, fmt: &[wchar_t], v: &mut Va) -> Res {
    errno::set_raw(0);
    let rc = x::vswprintf(buf.as_mut_ptr(), max, fmt.as_ptr(), (&raw mut v.ap).cast::<c_void>());
    let err = if rc < 0 { errno::get() } else { 0 };
    let n = usize::try_from(rc).unwrap_or(0).min(max.saturating_sub(1));
    Res { rc, err, hash: fnv(buf[..n].iter().flat_map(|w| w.to_ne_bytes())) }
}

/// Run `call` with allocation allowed, then with every allocation refused (each run with its
/// own `va_list` over `args`, built before): the first result, the allocator calls of the first
/// run, the second result and the refused attempts.
fn twice(args: &[A], mut call: impl FnMut(&mut Va) -> Res) -> (Res, u64, Res, u64) {
    let (mut v1, mut v2) = (va(args), va(args));
    let c0 = failinject::calls();
    let free = call(&mut v1);
    let calls = failinject::calls() - c0;
    let i0 = failinject::injected();
    failinject::fail_at_least(1);
    let refused = call(&mut v2);
    failinject::off();
    (free, calls, refused, failinject::injected() - i0)
}

/// The reference's allocation behaviour for a row.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ref {
    /// No allocation: libniji makes none either and gives the same result under refusal.
    None,
    /// The reference allocates: libniji may (refused: `-1`/`ENOMEM`), recorded only.
    Allocates,
}

fn nul(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

fn wnul(s: &str) -> Vec<wchar_t> {
    s.chars().map(|c| c as wchar_t).chain(core::iter::once(0)).collect()
}

fn positional(n: usize) -> String {
    (1..=n).map(|i| format!("%{i}$d ")).collect()
}

fn sequential(n: usize) -> String {
    "%d ".repeat(n)
}

/// Check a row; a failure is collected, with the row's numbers, into `bad`.
fn judge(bad: &mut Vec<String>, kind: &str, name: &str, r: Ref, (free, calls, refused, tried): (Res, u64, Res, u64)) {
    let line = format!("{kind} {name}: calls={calls} rc={} errno={} | refused: rc={} errno={} tried={tried} same={}", free.rc, free.err, refused.rc, refused.err, free == refused);
    println!("{line}");
    if r == Ref::None && (calls != 0 || tried != 0 || free != refused) {
        bad.push(line);
    }
}

#[test]
fn printf_no_allocation_where_the_reference_makes_none() {
    // Trace: s12_6_f_2 (ENOMEM only where memory is needed; black-box probe record above).
    let longs: Vec<u8> = nul(&"a".repeat(100_000));
    let longw: Vec<wchar_t> = wnul(&"b".repeat(10_000));
    let hello = nul("hello");
    let wshort = wnul("wide");
    let mut buf = vec![0u8; BIG];
    let mut wbuf: Vec<wchar_t> = vec![0; BIG / 4];
    let p = |v: &[u8]| A::I(v.as_ptr() as u64);
    let pw = |v: &[wchar_t]| A::I(v.as_ptr() as u64);
    let hundred: Vec<A> = (1..=100).map(A::I).collect();
    let mut rows: Vec<(String, String, Vec<A>, Ref)> = vec![
        ("%d".into(), "%d".into(), vec![A::I(42)], Ref::None),
        ("%s".into(), "%s".into(), vec![p(&hello)], Ref::None),
        ("%x %o %u".into(), "%x %o %u".into(), vec![A::I(255), A::I(8), A::I(7)], Ref::None),
        ("%5.2f".into(), "%5.2f".into(), vec![A::D(2.71875)], Ref::None),
        ("%e".into(), "%e".into(), vec![A::D(1e308)], Ref::None),
        ("%g 1e308".into(), "%g".into(), vec![A::D(1e308)], Ref::None),
        ("%a 1e308".into(), "%a".into(), vec![A::D(1e308)], Ref::None),
        ("%f 1e308".into(), "%f".into(), vec![A::D(1e308)], Ref::None),
        ("%f -1e308".into(), "%f".into(), vec![A::D(-1e308)], Ref::None),
        ("%Lf 2^1000".into(), "%Lf".into(), vec![pow2l(1000)], Ref::None),
        ("%Lf 2^13000".into(), "%Lf".into(), vec![pow2l(13000)], Ref::None),
        ("%La 2^13000".into(), "%La".into(), vec![pow2l(13000)], Ref::None),
        ("%.400e".into(), "%.400e".into(), vec![A::D(1.0)], Ref::None),
        ("%.10000f".into(), "%.10000f".into(), vec![A::D(1.0)], Ref::None),
        ("%.100000f".into(), "%.100000f".into(), vec![A::D(1.0)], Ref::Allocates),
        ("%.100000e".into(), "%.100000e".into(), vec![A::D(3.0)], Ref::Allocates),
        ("%.1100f 1e-300".into(), "%.1100f".into(), vec![A::D(1e-300)], Ref::None),
        ("%'f 1e308".into(), "%'f".into(), vec![A::D(1e308)], Ref::None),
        ("%0400f".into(), "%0400f".into(), vec![A::D(1.5)], Ref::None),
        ("%-400e".into(), "%-400e".into(), vec![A::D(1.5)], Ref::None),
        ("%+.300f".into(), "%+.300f".into(), vec![A::D(-2.5)], Ref::None),
        ("%.500d".into(), "%.500d".into(), vec![A::I(7)], Ref::None),
        ("%.100000d".into(), "%.100000d".into(), vec![A::I(7)], Ref::None),
        ("%.100000x".into(), "%.100000x".into(), vec![A::I(255)], Ref::None),
        ("%#.600o".into(), "%#.600o".into(), vec![A::I(8)], Ref::None),
        ("%+0600.500d".into(), "%+0600.500d".into(), vec![A::I(-7i64 as u64)], Ref::None),
        ("%100000d".into(), "%100000d".into(), vec![A::I(7)], Ref::None),
        ("%-100000d".into(), "%-100000d".into(), vec![A::I(7)], Ref::None),
        ("%0100000d".into(), "%0100000d".into(), vec![A::I(-7i64 as u64)], Ref::None),
        ("%'d".into(), "%'d".into(), vec![A::I(1_234_567)], Ref::None),
        ("%'.500d".into(), "%'.500d".into(), vec![A::I(1_234_567)], Ref::None),
        ("%100000s".into(), "%100000s".into(), vec![p(&hello)], Ref::None),
        ("%s long".into(), "%s".into(), vec![p(&longs)], Ref::None),
        ("%.50000s long".into(), "%.50000s".into(), vec![p(&longs)], Ref::None),
        ("%ls".into(), "%ls".into(), vec![pw(&wshort)], Ref::None),
        ("%ls long".into(), "%ls".into(), vec![pw(&longw)], Ref::None),
        ("%lc".into(), "%lc".into(), vec![A::I(u64::from('x'))], Ref::None),
        ("%300c".into(), "%300c".into(), vec![A::I(u64::from(b'c'))], Ref::None),
        ("%p".into(), "%p".into(), vec![p(&hello)], Ref::None),
        ("%*d".into(), "%*d".into(), vec![A::I(100_000), A::I(5)], Ref::None),
        ("%.*d".into(), "%.*d".into(), vec![A::I(100_000), A::I(5)], Ref::None),
        ("%m".into(), "%m".into(), vec![], Ref::None),
        ("%.300a".into(), "%.300a".into(), vec![A::D(1.0)], Ref::None),
        ("%.5000a".into(), "%.5000a".into(), vec![A::D(1.0)], Ref::None),
        // Sequential formats the simple-format path does not take: parsed a part at a time.
        ("grp seq 100".into(), "%'d ".repeat(100), hundred.clone(), Ref::None),
        ("star seq 50".into(), "%*d ".repeat(50), hundred.clone(), Ref::None),
        ("prec seq 100".into(), "%+.3d|".repeat(100), hundred.clone(), Ref::None),
    ];
    for n in [8, 12] {
        rows.push((format!("pos {n}"), positional(n), hundred.clone(), Ref::None));
    }
    for n in [16, 24, 100] {
        rows.push((format!("pos {n}"), positional(n), hundred.clone(), Ref::Allocates));
    }
    for n in [8, 16, 32, 64, 100] {
        rows.push((format!("seq {n}"), sequential(n), hundred.clone(), Ref::None));
    }
    let mut bad = Vec::new();
    for (name, f, args, r) in &rows {
        let f = nul(f);
        judge(&mut bad, "N", name, *r, twice(args, |v| narrow(&mut buf, BIG, &f, v)));
    }
    // Wide: (name, format, arguments, buffer units, reference).
    let wrows: Vec<(String, String, Vec<A>, usize, Ref)> = vec![
        ("%d".into(), "%d".into(), vec![A::I(42)], 256, Ref::None),
        ("%s".into(), "%s".into(), vec![p(&hello)], 256, Ref::None),
        ("%ls".into(), "%ls".into(), vec![pw(&wshort)], 256, Ref::None),
        ("%lc".into(), "%lc".into(), vec![A::I(u64::from('x'))], 256, Ref::None),
        ("%f 1e308".into(), "%f".into(), vec![A::D(1e308)], BIG / 4, Ref::None),
        ("%Lf 2^13000".into(), "%Lf".into(), vec![pow2l(13000)], BIG / 4, Ref::None),
        ("%.400e".into(), "%.400e".into(), vec![A::D(1.0)], BIG / 4, Ref::None),
        ("%.10000f".into(), "%.10000f".into(), vec![A::D(1.0)], BIG / 4, Ref::None),
        ("%.500d".into(), "%.500d".into(), vec![A::I(7)], BIG / 4, Ref::None),
        ("%.100000d".into(), "%.100000d".into(), vec![A::I(7)], BIG / 4, Ref::None),
        ("%100000d big".into(), "%100000d".into(), vec![A::I(7)], BIG / 4, Ref::None),
        ("%100000d 256".into(), "%100000d".into(), vec![A::I(7)], 256, Ref::None),
        ("%300d 256".into(), "%300d".into(), vec![A::I(7)], 256, Ref::None),
        ("hello 4".into(), "hello".into(), vec![], 4, Ref::None),
        ("%s 4".into(), "%s".into(), vec![p(&hello)], 4, Ref::None),
        ("%ls long".into(), "%ls".into(), vec![pw(&longw)], BIG / 4, Ref::None),
        ("%s long".into(), "%s".into(), vec![p(&longs)], BIG / 4, Ref::None),
        ("pos 12".into(), positional(12), hundred.clone(), 4096, Ref::None),
        ("pos 24".into(), positional(24), hundred.clone(), 4096, Ref::Allocates),
        ("seq 100".into(), sequential(100), hundred.clone(), 4096, Ref::None),
        ("%m".into(), "%m".into(), vec![], 256, Ref::None),
    ];
    for (name, f, args, max, r) in &wrows {
        let f = wnul(f);
        judge(&mut bad, "W", name, *r, twice(args, |v| wide(&mut wbuf, *max, &f, v)));
    }
    assert!(bad.is_empty(), "rows that allocate where the reference does not:\n{}", bad.join("\n"));
}

/// `swprintf` whose result does not fit: -1 and `E2BIG`, whatever the size (black-box probe of
/// the reference: the same `errno` for `L"hello"` into 4 units and `L"%100000d"` into 256).
#[test]
fn swprintf_result_does_not_fit_is_e2big() {
    // Trace: s12_6_f_2 (no spec clause names this errno; black-box probe record above).
    let mut wbuf: Vec<wchar_t> = vec![0; 256];
    for (f, args, max) in [("hello", vec![], 4usize), ("%100000d", vec![A::I(7)], 256), ("%.300d", vec![A::I(7)], 256)] {
        let r = wide(&mut wbuf, max, &wnul(f), &mut va(&args));
        assert_eq!((r.rc, r.err), (-1, 7), "swprintf {f:?} into {max} units");
    }
}

/// The text of one narrow call (allocation allowed).
fn text_of(fmt: &str, args: &[A]) -> String {
    let mut buf = vec![0u8; BIG];
    let mut v = va(args);
    let f = nul(fmt);
    errno::set_raw(0);
    let rc = x::vsnprintf(buf.as_mut_ptr().cast::<c_char>(), BIG, f.as_ptr().cast::<c_char>(), (&raw mut v.ap).cast::<c_void>());
    assert!(rc >= 0, "{fmt}: rc {rc} errno {}", errno::get());
    String::from_utf8(buf[..rc as usize].to_vec()).unwrap()
}

/// The text of one wide call into a large buffer.
fn wtext_of(fmt: &str, args: &[A]) -> String {
    let mut buf: Vec<wchar_t> = vec![0; BIG / 4];
    let mut v = va(args);
    let f = wnul(fmt);
    let rc = x::vswprintf(buf.as_mut_ptr(), BIG / 4, f.as_ptr(), (&raw mut v.ap).cast::<c_void>());
    assert!(rc >= 0, "{fmt}: rc {rc}");
    buf[..rc as usize].iter().map(|&w| char::from_u32(w as u32).unwrap()).collect()
}

/// Fields longer than the inline room are produced straight to the output: the same text as
/// one built here from the conversion's definition (padding after the sign for `0`, spaces
/// before or after, precision zeros, the exact decimal expansion of a large value).
#[test]
fn long_fields_have_their_full_text() {
    // Trace: s12_6_c_5 (width and precision), s12_6_g_1 (exact floating output), s12_6_g_2 (any destination).
    let z = |n: usize| "0".repeat(n);
    let sp = |n: usize| " ".repeat(n);
    let rows: Vec<(&str, Vec<A>, String)> = vec![
        ("%.500d", vec![A::I(7)], z(499) + "7"),
        ("%.100000x", vec![A::I(255)], z(99_998) + "ff"),
        ("%#.600o", vec![A::I(8)], z(598) + "10"),
        ("%+0600.500d", vec![A::I(-7i64 as u64)], sp(99) + "-" + &z(499) + "7"),
        ("%0100000d", vec![A::I(-7i64 as u64)], "-".to_string() + &z(99_998) + "7"),
        ("%-100000d", vec![A::I(7)], "7".to_string() + &sp(99_999)),
        // Left-justified fields whose own text is long: the padding follows the long text.
        ("%-600.500d|", vec![A::I(7)], z(499) + "7" + &sp(100) + "|"),
        ("%-400.300f|", vec![A::D(1.0)], "1.".to_string() + &z(300) + &sp(98) + "|"),
        ("%100000d", vec![A::I(7)], sp(99_999) + "7"),
        ("%#0300x", vec![A::I(255)], "0x".to_string() + &z(296) + "ff"),
        ("%.10000f", vec![A::D(1.0)], "1.".to_string() + &z(10_000)),
        ("%0400f", vec![A::D(1.5)], z(392) + "1.500000"),
        ("%-400e", vec![A::D(1.5)], "1.500000e+00".to_string() + &sp(388)),
        ("%+.300f", vec![A::D(-2.5)], "-2.5".to_string() + &z(299)),
        ("%f", vec![A::D(1e308)], format!("{:.6}", 1e308f64)),
        ("%0400f", vec![A::D(-1e300)], "-".to_string() + &z(399 - format!("{:.6}", 1e300f64).len()) + &format!("{:.6}", 1e300f64)),
        ("%+500.0f", vec![A::D(1e300)], sp(499 - format!("{:.0}", 1e300f64).len()) + "+" + &format!("{:.0}", 1e300f64)),
        ("%Lf", vec![pow2l(1000)], format!("{:.6}", 2f64.powi(1000))),
        ("%.5000a", vec![A::D(1.0)], "0x1.".to_string() + &z(5000) + "p+0"),
        ("%-300.3s|", vec![A::I(c"abcdef".as_ptr() as u64)], "abc".to_string() + &sp(297) + "|"),
    ];
    for (f, args, want) in &rows {
        assert_eq!(text_of(f, args), *want, "{f}");
        assert_eq!(wtext_of(f, args), *want, "wide {f}");
    }
    // Sequential formats parsed a part at a time, and `%n` counting across the parts.
    let hundred: Vec<A> = (1..=100).map(A::I).collect();
    let want: String = (1..=100).map(|i| format!("{i:+04}|")).collect();
    assert_eq!(text_of(&"%+04d|".repeat(100), &hundred), want);
    assert_eq!(wtext_of(&"%d ".repeat(100), &hundred), (1..=100).map(|i| format!("{i} ")).collect::<String>());
    let mut count: i32 = -1;
    let mut args: Vec<A> = (1..=40).map(A::I).collect();
    args.push(A::I(&raw mut count as u64));
    let f = "%'d,".repeat(40) + "%n";
    let t = text_of(&f, &args);
    assert_eq!((t.len(), count as usize), ((1..=40).map(|i| format!("{i},").len()).sum::<usize>(), t.len()));
    // Positional past the bounded parse's room.
    let pos: String = (1..=20).rev().map(|i| format!("%{i}$d ")).collect();
    assert_eq!(text_of(&pos, &hundred[..20]), (1..=20).rev().map(|i| format!("{i} ")).collect::<String>());
}
