//! Deeply nested words in `wordexp` (HL-14, PATTERN-NESTING): the parser, the expander and the
//! arithmetic evaluator keep their nesting on stacks on the heap, so `${...}`, `$((...))` and
//! parentheses nest as deep as the text's length allows (memory, the process's limits, is the
//! only bound), never using the thread stack for it. The old limits were 128 nested expansions
//! and 512 arithmetic levels; the reference accepts 300 nested `${x:-...}` (audit probe).
//! Running out of memory is `WRDE_NOSPACE`, never a crash.
#![allow(clippy::arithmetic_side_effects, clippy::unwrap_used, clippy::indexing_slicing, clippy::panic, missing_docs)]

use niji::error::Result;
use niji::locale::Locale;
use niji::mem::TryVec;
use niji::pattern::wordexp::{self, Shell, ValBuf, WordError, Words, NOCMD};
use niji_boundary::selftest::failinject::{self, FailAlloc};

#[global_allocator]
static ALLOC: FailAlloc = FailAlloc;

/// A shell with `x=abc` and `y=1`; nothing else set. Assignments are recorded, not stored
/// (the allocation-failure sweep runs no assignment).
#[derive(Default)]
struct Sh {
    assigned: Vec<(Vec<u8>, Vec<u8>)>,
}

impl Shell for Sh {
    fn var(&mut self, name: &[u8], out: &mut ValBuf) -> Result<bool> {
        out.clear();
        let v: &[u8] = match name {
            b"x" => b"abc",
            b"y" => b"1",
            _ => return Ok(false),
        };
        out.extend_from_slice(v)?;
        Ok(true)
    }
    fn set_var(&mut self, name: &[u8], value: &[u8]) -> Result<()> {
        self.assigned.push((name.to_vec(), value.to_vec()));
        Ok(())
    }
    fn arg_count(&mut self) -> usize {
        0
    }
    fn arg(&mut self, _i: usize, out: &mut ValBuf) -> Result<bool> {
        out.clear();
        Ok(false)
    }
    fn pid(&mut self) -> i32 {
        4242
    }
    fn home(&mut self, _user: Option<&[u8]>, out: &mut ValBuf) -> Result<bool> {
        out.clear();
        Ok(false)
    }
    fn run(&mut self, _command: &[u8], _show_err: bool, out: &mut TryVec<u8>) -> Result<()> {
        out.clear();
        Ok(())
    }
    fn stderr(&mut self, _text: &[u8]) {}
    fn glob(&mut self, _pattern: &[u8], _out: &mut TryVec<TryVec<u8>>) -> Result<()> {
        Ok(())
    }
}

struct Fields(Vec<String>);

impl Words for Fields {
    fn push(&mut self, word: &[u8]) -> Result<()> {
        self.0.push(String::from_utf8_lossy(word).into_owned());
        Ok(())
    }
}

/// The fields of `text`, or the error.
fn we(text: &str) -> core::result::Result<Vec<String>, WordError> {
    let mut f = Fields(Vec::new());
    wordexp::wordexp(text.as_bytes(), NOCMD, &Locale::C, &mut Sh::default(), &mut f).map(|()| f.0)
}

/// `n` levels of `open` around `mid`, each closed by `close`.
fn nest(open: &str, mid: &str, close: &str, n: usize) -> String {
    format!("{}{mid}{}", open.repeat(n), close.repeat(n))
}

/// Every kind of nesting `n` deep, past the old limits.
fn every_kind(n: usize) {
    let one = |s: &str| we(s).unwrap_or_else(|e| panic!("{n} deep: {e:?}"));
    // Defaults of unset variables around a word: unquoted it splits, quoted it does not.
    assert_eq!(one(&nest("${u:-", "a b", "}", n)), ["a", "b"], "{n} nested ${{u:-...}}");
    assert_eq!(one(&format!("\"{}\"", nest("${u:-", "a b", "}", n))), ["a b"], "{n} nested ${{u:-...}} quoted");
    // Double quotes inside every level's word.
    assert_eq!(one(&nest("${u:-\"", "a b", "\"}", n)), ["a b"], "{n} nested ${{u:-\"...\"}}");
    // Alternatives of a set variable, the innermost a value.
    assert_eq!(one(&nest("${y:+", "$x", "}", n)), ["abc"], "{n} nested ${{y:+...}}");
    // Text before a nested word stays in its own word at every level: the word of a set
    // variable's `${y:-b}` is not used, so its `b` must not join the enclosing word's `a`.
    assert_eq!(one(&format!("{}m{}", "${u:-a${y:-b}".repeat(n), "}".repeat(n))), ["a1".repeat(n) + "m"], "{n} nested words after text");
    // A pattern removal whose pattern is a nested word.
    assert_eq!(one(&format!("${{x%{}}}", nest("${u:-", "c", "}", n))), ["ab"], "{n} deep %");
    // Arithmetic with an expansion at every level: n additions of 1.
    let mut s = String::from("$y");
    for _ in 0..n {
        s = format!("$(({s}+1))");
    }
    assert_eq!(one(&s), [(n + 1).to_string()], "{n} nested $((...))");
    // Parentheses, unary operators and conditionals inside one expansion.
    assert_eq!(one(&format!("$(({}))", nest("(", "1+2", ")", n))), ["3"], "{n} parentheses");
    assert_eq!(one(&format!("$(({}1))", "- ".repeat(n))), [if n % 2 == 0 { "1" } else { "-1" }], "{n} unary minus");
    assert_eq!(one(&format!("$(({}7))", "0?1:".repeat(n))), ["7"], "{n} conditionals");
    assert_eq!(one(&format!("$(({}0))", "1?".repeat(n) + &"0:".repeat(n))), ["0"], "{n} nested middle operands");
    // A side not taken is parsed, not evaluated: no error from its division by zero.
    assert_eq!(one(&format!("$((0&&({})))", nest("(", "1/0", ")", n))), ["0"], "{n} deep short circuit");
}

#[test]
fn words_nest_past_the_old_depth() {
    // Trace: s14_10_c_1 (parameter and arithmetic expansion: a word in a `${...}` is expanded,
    // and may hold expansions itself); HL-14.
    for n in [129, 600, 1000] {
        every_kind(n);
    }
    // Errors are found however deep they are.
    assert_eq!(we(&nest("${u:-", "${", "}", 1000)), Err(WordError::Syntax), "unclosed, 1000 deep");
    assert_eq!(we(&format!("$(({}1+))", "(".repeat(1000))), Err(WordError::Syntax), "unbalanced, 1000 deep");
    // An assignment of a deep word assigns its value.
    let mut sh = Sh::default();
    let mut f = Fields(Vec::new());
    wordexp::wordexp(format!("${{v={}}}", nest("${u:-", "w", "}", 1000)).as_bytes(), NOCMD, &Locale::C, &mut sh, &mut f).unwrap();
    assert_eq!((f.0, sh.assigned), (vec!["w".to_string()], vec![(b"v".to_vec(), b"w".to_vec())]), "deep ${{v=...}}");
}

#[test]
fn deep_words_run_on_a_small_stack() {
    // Trace: no spec clause (resource bound of the implementation: nesting depth costs heap, not
    // thread stack; HL-14).
    let t = std::thread::Builder::new().stack_size(64 << 10).spawn(|| every_kind(1000));
    t.unwrap().join().unwrap();
}

#[test]
fn deep_words_out_of_memory_are_nospace() {
    // Trace: s14_10_f_0 (`WRDE_NOSPACE`: an allocation failed); ruling #5594 (running out is a
    // clean error from a fallible allocation); HL-14.
    let mut s = String::from("$y");
    for _ in 0..40 {
        s = format!("$(({s}+1))");
    }
    let text = format!("{} \"{}\" ${{x%{}}} $(({}))", nest("${u:-", "a b", "}", 200), nest("${y:+", "$x", "}", 200), nest("${u:-", "c", "}", 100), nest("(", &s, ")", 300));
    struct Count(usize);
    impl Words for Count {
        fn push(&mut self, _word: &[u8]) -> Result<()> {
            self.0 += 1;
            Ok(())
        }
    }
    for n in 1..100_000u64 {
        let hit = failinject::injected();
        let mut count = Count(0);
        let mut sh = Sh::default();
        failinject::fail_nth(n);
        let r = wordexp::wordexp(text.as_bytes(), NOCMD, &Locale::C, &mut sh, &mut count);
        failinject::off();
        if failinject::injected() == hit {
            assert_eq!((r, count.0), (Ok(()), 5), "without an injected failure (allocation {n} not reached)");
            assert!(n > 1, "no allocation was failed");
            return;
        }
        assert_eq!(r, Err(WordError::NoSpace), "allocation {n} failed");
    }
    panic!("over 100,000 allocations");
}
