//! The arithmetic lexer of `wordexp` (WORDEXP-FIXED): constants and operator spellings, including
//! the refusals, which the fast paths (direct operator match, one-pass decimal constants) must
//! keep. Clauses: s14_10_c_1, s14_10_c_2 (D-14.5).
#![allow(clippy::arithmetic_side_effects, clippy::unwrap_used, clippy::indexing_slicing, missing_docs)]

use niji::error::Result;
use niji::pattern::wordexp::{self, ValBuf, WordError};

fn eval(text: &str) -> core::result::Result<i64, WordError> {
    let mut var = |name: &[u8], out: &mut ValBuf| -> Result<bool> {
        out.clear();
        if name == b"x" {
            out.extend_from_slice(b"7")?;
            Ok(true)
        } else {
            Ok(false)
        }
    };
    wordexp::arith::evaluate(text.as_bytes(), false, &mut var)
}

#[test]
fn constants_keep_their_radix_and_range() {
    // Clauses: s14_10_c_1 (D-14.5: constants, whole, in their radix).
    assert_eq!(eval("1234567"), Ok(1_234_567));
    assert_eq!(eval("9223372036854775807"), Ok(i64::MAX));
    assert_eq!(eval("9223372036854775808"), Err(WordError::Syntax));
    assert_eq!(eval("99999999999999999999"), Err(WordError::Syntax));
    assert_eq!(eval("010"), Ok(8));
    assert_eq!(eval("0x1F"), Ok(31));
    assert_eq!(eval("12ab"), Err(WordError::Syntax));
    assert_eq!(eval("12_"), Err(WordError::Syntax));
    assert_eq!(eval("08"), Err(WordError::Syntax));
    assert_eq!(eval("x*6"), Ok(42));
}

#[test]
fn operator_spellings_and_refusals() {
    // Clauses: s14_10_c_1 (D-14.5: the operators; assignment, ++, --, ** and , refused).
    assert_eq!(eval("3<4"), Ok(1));
    assert_eq!(eval("3>4"), Ok(0));
    assert_eq!(eval("3<=3"), Ok(1));
    assert_eq!(eval("3>=4"), Ok(0));
    assert_eq!(eval("1<<4"), Ok(16));
    assert_eq!(eval("256>>4"), Ok(16));
    assert_eq!(eval("3==3"), Ok(1));
    assert_eq!(eval("3!=3"), Ok(0));
    assert_eq!(eval("1&&2"), Ok(1));
    assert_eq!(eval("0||0"), Ok(0));
    assert_eq!(eval("6&3|8^1"), Ok(11));
    assert_eq!(eval("(1)?(2):(3)"), Ok(2));
    assert_eq!(eval("~0"), Ok(-1));
    assert_eq!(eval("!5"), Ok(0));
    assert_eq!(eval("-(-3)"), Ok(3));
    assert_eq!(eval(" 7 % 4 * 2 "), Ok(6));
    for bad in ["1<<=2", "1>>=2", "x+=1", "x-=1", "x*=1", "x/=1", "x%=1", "x&=1", "x^=1", "x|=1", "x=1", "1,2", "x++", "x--", "2**3", "1 @ 2", "1 )", "(1"] {
        assert_eq!(eval(bad), Err(WordError::Syntax), "{bad}");
    }
}

#[test]
fn precedence_conditionals_and_short_circuit() {
    // Clauses: s14_10_c_1 (the evaluator, PATTERN-NESTING-3: operators wait on a stack by
    // precedence; binary operators group to the left, `?:` to the right, the side not taken is
    // parsed but not evaluated).
    let cases: &[(&str, i64)] = &[
        ("1+2*3", 7),
        ("2*3+1", 7),
        ("1-2-3", -4),
        ("100/10/5", 2),
        ("7-1*2-3", 2),
        ("1+2<<1", 6),
        ("1|2^3&4", 3),
        ("2+3*4==14&&1", 1),
        ("-2*3", -6),
        ("!0+1", 2),
        ("~1+1", -1),
        ("-(1+2)*2", -6),
        ("-x", -7),
        ("1?0:1?2:3", 0),
        ("0?1:0?2:3", 3),
        ("1?0?4:5:6", 5),
        ("1?2?3:4?5:6:7", 3),
        ("1?2:3+4", 2),
        ("0?2:3+4", 7),
        ("1+1?5:6", 5),
        ("(0?1:2)*3", 6),
        ("0&&1/0", 0),
        ("1||1/0", 1),
        ("0?1/0:2", 2),
        ("1?2:1/0", 2),
        ("0&&1?1/0:4", 4),
        ("0?(1?1/0:1/0):5", 5),
        ("0&&(1/0+(2?1/0:3))", 0),
        ("0&&9223372036854775807+1", 0),
    ];
    for &(text, v) in cases {
        assert_eq!(eval(text), Ok(v), "{text}");
    }
    for bad in ["1&&1/0", "0||1/0", "0?1/0:1/0", "1&&9223372036854775807+1", "1?2", "1:2", "(1:2)", "1?2:", "()", ")", "(1?2:3", "1?(2:3)", "1+", "*1", "1 2"] {
        assert_eq!(eval(bad), Err(WordError::Syntax), "{bad}");
    }
    // Deeper than the stack's inline part: parentheses and operators, nested conditionals.
    let n = 40i64;
    let text = (1..n).map(|k| format!("{k}-(")).collect::<String>() + &n.to_string() + &")".repeat(39);
    let want = (1..n).rev().fold(n, |acc, k| k - acc);
    assert_eq!(eval(&text), Ok(want), "{text}");
    assert_eq!(eval(&("1?".repeat(40) + "7" + &":0".repeat(40))), Ok(7));
    assert_eq!(eval(&("0?0:".repeat(40) + "7")), Ok(7));
    assert_eq!(eval(&("1?0:".repeat(40) + "7")), Ok(0));
}
