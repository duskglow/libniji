//! Deeply nested extended patterns (`FNM_EXTMATCH`; HL-13, PATTERN-NESTING): the compiler and the
//! matcher keep their work on heap stacks, so groups nest as deep as the pattern's length allows
//! (memory, the process's limits, is the only bound) and never use the thread stack for it; a
//! group nested in another is evaluated at most once per start position, so the work stays
//! polynomial whatever the nesting. The reference matches 1,000 nested `@(a)` against `a`
//! (probed; the PATTERN-NESTING handoff). Running out of memory is `ENOMEM`, never a crash.
#![allow(clippy::arithmetic_side_effects, clippy::unwrap_used, clippy::indexing_slicing, clippy::panic, missing_docs)]

use niji::error::Errno;
use niji::locale::Locale;
use niji::pattern::fnmatch::{self as fm, Pattern};
use niji_boundary::selftest::failinject::{self, FailAlloc};

#[global_allocator]
static ALLOC: FailAlloc = FailAlloc;

const X: i32 = fm::EXTMATCH;

/// Work counts (`Pattern::work`) of `!(*` nested `d` deep around `b` over 24 `a`s: exact counts
/// of today's matcher; a bound one step lower fails.
const COMPLEMENTS_4: u64 = 1804;
const COMPLEMENTS_8: u64 = 4604;

fn nested(open: &str, mid: &str, close: &str, n: usize) -> Vec<u8> {
    format!("{}{mid}{}", open.repeat(n), close.repeat(n)).into_bytes()
}

/// The pattern compiled once (compiling scans a group's text once per group around it).
fn pat(p: &[u8]) -> Pattern {
    Pattern::new(p, X, &Locale::C).unwrap()
}

fn m(p: &Pattern, s: &str) -> bool {
    p.matches(s.as_bytes(), &Locale::C).unwrap()
}

/// `n` nested groups of each kind, past the old limit (64) and the reference's probed depth.
fn every_kind(n: usize) {
    let nested = |o, i, c, n| pat(&nested(o, i, c, n));
    let one = nested("@(", "a", ")", n);
    assert!(m(&one, "a") && !m(&one, "b") && !m(&one, "aa") && !m(&one, ""), "{n} nested @(...) around a");
    // An alternative at every level: `@(x|@(x|...@(x|a)...))` is `x` or `a`.
    let alt = nested("@(x|", "a", ")", n);
    assert!(m(&alt, "a") && m(&alt, "x") && !m(&alt, "xa") && !m(&alt, "b"), "{n} nested @(x|...)");
    // Two complements are the pattern itself: an even number of `!(...)` around `a` is `a`, an odd number
    // anything else.
    let even = nested("!(", "a", ")", n & !1);
    assert!(m(&even, "a") && !m(&even, "b") && !m(&even, ""), "{} nested !(...)", n & !1);
    let odd = nested("!(", "a", ")", n | 1);
    assert!(!m(&odd, "a") && m(&odd, "b") && m(&odd, "") && m(&odd, "aa"), "{} nested !(...)", n | 1);
    // Repetitions of repetitions: any number of `ab`; `+(...)` at least one.
    let many = nested("*(", "ab", ")", n);
    assert!(m(&many, "") && m(&many, "ababab") && !m(&many, "aba"), "{n} nested *(...)");
    let some = nested("+(", "ab", ")", n);
    assert!(!m(&some, "") && m(&some, "abab") && !m(&some, "b"), "{n} nested +(...)");
    let opt = nested("?(", "ab", ")", n);
    assert!(m(&opt, "") && m(&opt, "ab") && !m(&opt, "abab"), "{n} nested ?(...)");
}

#[test]
fn groups_nest_past_the_old_depth() {
    // Trace: s14_7_c_2_6 (`FNM_EXTMATCH`: a group's alternatives are patterns, so groups nest); HL-13.
    for n in [65, 1000] {
        every_kind(n);
    }
    // A deep group among the pattern's own items, in a multibyte locale (the characters path).
    let u = Locale::c_utf8(b"C.UTF-8");
    let p = [b"x*".as_slice(), &nested("@(\u{e9}|", "a", ")", 1000), b"!(z)"].concat();
    assert!(fm::fnmatch(&p, "x\u{e9}y".as_bytes(), X, &u).unwrap() && !fm::fnmatch(&p, "x\u{e9}z".as_bytes(), X, &u).unwrap());
}

#[test]
fn nested_complements_take_polynomial_work() {
    // Trace: no spec clause (an implementation bound: a nested group is evaluated at most once
    // per start position, so the work is polynomial in the pattern and the string; HL-13).
    // `!(*!(*...!(*b)...))` over `a`s: every group is tried from every position the `*` before
    // it reaches. With no `b`, one level matches everything, two nothing, and so on.
    let s = "a".repeat(24);
    let run = |d: usize| {
        let p = Pattern::new(&nested("!(*", "b", ")", d), X, &Locale::C).unwrap();
        let r = p.matches(s.as_bytes(), &Locale::C).unwrap();
        assert_eq!(r, d % 2 == 1, "{d} nested !(*...)");
        p.work()
    };
    let w4 = run(4);
    eprintln!("WORK nested complements 4: {w4}");
    assert!(w4 <= COMPLEMENTS_4, "4 nested complements: {w4} units, over {COMPLEMENTS_4}");
    let w8 = run(8);
    eprintln!("WORK nested complements 8: {w8}");
    assert!(w8 <= COMPLEMENTS_8, "8 nested complements: {w8} units, over {COMPLEMENTS_8}");
    // Linear in the depth here (each level's entries once), not a power of the string's length.
    assert!(w8 <= 3 * w4, "doubling the depth: {w4} -> {w8} units");
}

#[test]
fn deep_patterns_run_on_a_small_stack() {
    // Trace: no spec clause (resource bound of the implementation: nesting depth costs heap, not
    // thread stack; HL-13).
    let t = std::thread::Builder::new().stack_size(64 << 10).spawn(|| {
        every_kind(1000);
        // Compiled, matched and dropped on this stack.
        let p = pat(&nested("*(@(x|", "a", "))", 3000));
        assert!(p.matches(b"xaax", &Locale::C).unwrap() && !p.matches(b"xb", &Locale::C).unwrap());
        drop(p);
    });
    t.unwrap().join().unwrap();
}

#[test]
fn deep_patterns_out_of_memory_are_enomem() {
    // Trace: s14_7_f_0 (`fnmatch` fails only for lack of memory); ruling #5594 (running out is a
    // clean error from a fallible allocation); HL-13.
    let p = nested("!(x|", "*(a)", ")", 200);
    for n in 1..100_000u64 {
        let hit = failinject::injected();
        failinject::fail_nth(n);
        let r = fm::fnmatch(&p, b"aaaa", X, &Locale::C);
        failinject::off();
        if failinject::injected() == hit {
            assert_eq!(r, Ok(true), "without an injected failure (allocation {n} not reached)");
            assert!(n > 1, "no allocation was failed");
            return;
        }
        assert_eq!(r, Err(Errno::ENOMEM), "allocation {n} failed");
    }
    panic!("over 100,000 allocations");
}
