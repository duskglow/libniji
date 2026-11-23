// Specified from spec §21 (phase 6).
// Specify a test by replacing its stub with a real #[test] fn of the SAME NAME (see crates/spectest).
// Once a file contains specified tests, `cargo xtask stubs` stops rewriting it and only checks coverage.

//! Clause tests for spec §21 (11 clauses).
//! Jumps and contexts move the calling thread's stack: they are checked by C conformance
//! programs linked against the products.
#![allow(clippy::unwrap_used, missing_docs)]
#[path = "support/scratch.rs"]
mod scratch;

/// Build and run C conformance programs against the products (`conformance/run.sh`).
fn conformance(names: &[&str]) {
    static RUNNER: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _g = RUNNER.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let products = ["target/niji/libc.a", "target/niji/niji-crt1.o"];
    assert!(products.iter().all(|p| root.join(p).exists()), "the products are missing: run `cargo xtask so` (`cargo xtask test` does)");
    let o = std::process::Command::new("sh").arg(root.join("conformance/run.sh")).args(names).output().unwrap();
    assert!(o.status.success(), "{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
}

/// §21.1 Contract: **`setjmp(env)`** saves the calling environment (callee-saved registers, stack pointer, program counter, and for `sigsetjmp(env, 1)` the signal mask). It returns 0 when called directly.
#[test]
fn s21_1_c_1() {
    conformance(&["s21_setjmp"]);
}
/// §21.1 Contract: **`longjmp(env, v)`** restores the environment so that `setjmp` appears to return `v` (1 if `v` is 0). The function that called `setjmp` must not have returned.
#[test]
fn s21_1_c_2() {
    conformance(&["s21_setjmp"]);
}
/// §21.1 Contract: **`_setjmp`/`_longjmp`** never save or restore the mask. **`setjmp`** on glibc does not save the mask either (BSD would). ⚑ HIST: the default differs by Unix family.
#[test]
fn s21_1_c_3() {
    conformance(&["s21_setjmp"]);
}
/// §21.1 Contract: **`__longjmp_chk`** is the fortified form. It aborts if the jump would move to a frame *below* the current stack pointer (an invalid jump), except for jumps between the alternate signal stack and the 
#[test]
fn s21_1_c_4() {
    conformance(&["s21_setjmp", "s21_longjmp_chk"]);
}
/// §21.1 Inputs: `setjmp` may appear only in restricted syntactic contexts (as a whole expression statement, or in a comparison inside `if`/`while`/`switch`). Local non-`volatile` variables modified after `setjmp` hav
#[test]
fn s21_1_i_0() {
    conformance(&["s21_setjmp"]);
}
/// §21.1 Guarantees: On x86_64 the saved stack and return addresses are mangled with a per-process secret (pointer guard). With CET, the shadow stack is unwound to match (via `incssp`).
#[test]
fn s21_1_g_0() {
    // Pointer mangling (s22_more checks the saved stack pointer is not stored in the clear).
    // The shadow stack (D-21.5): s21_shadow_stack enables it and jumps, switches contexts, vforks
    // and runs threads under it; SKIPPED (printed by the program) where the CPU or kernel has none.
    conformance(&["s22_more", "s21_shadow_stack"]);
    shadow_stack_instructions();
}

/// The shadow-stack handling is in the shipped `libc.a` (D-21.5), checked on any host: the
/// program above SKIPS where the CPU or kernel has no shadow stack, so a routine that lost its
/// `incssp` (and would fault once the shadow stack is on) still passes there. Each routine that
/// records, pops or switches the shadow stack must contain the instructions D-21.5 gives it.
/// Only the members defining these routines are disassembled (the whole archive takes a minute).
/// Needs `nm`, `ar` and `objdump`; without them the check says so and passes.
fn shadow_stack_instructions() {
    use std::process::Command;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let libc = root.join("target/niji/libc.a");
    // objdump may label a routine by its library-internal alias (`__niji_libc_<name>`).
    let plain = |s: &str| s.strip_prefix("__niji_libc_").unwrap_or(s).to_string();
    // (names the routine may be listed under, instructions it must contain)
    let want: [(&[&str], &[&str]); 6] = [
        (&["niji_machine_sigsetjmp"], &["rdsspq"]),
        (&["niji_machine_longjmp_nomask"], &["rdsspq", "incsspq"]),
        (&["niji_machine_getcontext"], &["rdsspq"]),
        (&["niji_machine_swapcontext"], &["rdsspq"]),
        (&["niji_machine_setcontext"], &["rdsspq", "incsspq", "rstorssp", "saveprevssp"]),
        (&["vfork", "__vfork"], &["rdsspq", "incsspq"]),
    ];
    let nm = match Command::new("nm").args(["-A", "--defined-only"]).arg(&libc).output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("SKIP: shadow-stack instruction check: no nm ({e})");
            return;
        }
    };
    assert!(nm.status.success(), "nm libc.a: {}", String::from_utf8_lossy(&nm.stderr));
    // `<archive>:<member>:<value> <type> <symbol>`: the members that define a wanted routine.
    let mut members = std::collections::BTreeSet::new();
    for line in String::from_utf8_lossy(&nm.stdout).lines() {
        let (Some(head), Some(sym)) = (line.split_whitespace().next(), line.split_whitespace().last()) else { continue };
        if want.iter().any(|(names, _)| names.contains(&plain(sym).as_str())) {
            if let Some(m) = head.rsplit(':').nth(1) {
                members.insert(m.to_string());
            }
        }
    }
    let dir = scratch::Scratch::dir("shstk-members");
    let x = Command::new("ar").arg("x").arg(&libc).args(&members).current_dir(&*dir).status();
    let Ok(x) = x else {
        eprintln!("SKIP: shadow-stack instruction check: no ar");
        return;
    };
    assert!(x.success(), "ar x libc.a {members:?}");
    let o = match Command::new("objdump").args(["-d", "--no-show-raw-insn"]).args(members.iter().map(|m| dir.join(m))).output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("SKIP: shadow-stack instruction check: no objdump ({e})");
            return;
        }
    };
    assert!(o.status.success(), "objdump -d {members:?}: {}", String::from_utf8_lossy(&o.stderr));
    let text = String::from_utf8_lossy(&o.stdout);
    // Function name -> its instruction mnemonics (objdump starts each symbol with `<name>:`).
    let mut bodies: std::collections::HashMap<String, Vec<&str>> = std::collections::HashMap::new();
    let mut cur: Option<String> = None;
    for line in text.lines() {
        if let Some(name) = line.strip_suffix(">:").and_then(|l| l.split_once(" <")).map(|(_, n)| plain(n)) {
            bodies.entry(name.clone()).or_default();
            cur = Some(name);
        } else if let (Some(f), Some((_, insn))) = (&cur, line.split_once(":\t")) {
            if let Some(m) = insn.split_whitespace().next() {
                bodies.entry(f.clone()).or_default().push(m);
            }
        }
    }
    let mut missing = Vec::new();
    for (names, insns) in want {
        let body = names.iter().find_map(|n| bodies.get(*n));
        let Some(body) = body else {
            missing.push(format!("{}: not in libc.a", names.join("/")));
            continue;
        };
        for i in insns {
            if !body.contains(i) {
                missing.push(format!("{}: no {i}", names.join("/")));
            }
        }
    }
    assert!(missing.is_empty(), "libc.a routines without their shadow-stack handling (D-21.5): {missing:?}");
}
/// §21.2 Contract: **`getcontext(&uc)`** saves the full user context (registers, signal mask, stack description).
#[test]
fn s21_2_c_1() {
    conformance(&["s21_context"]);
}
/// §21.2 Contract: **`setcontext(&uc)`** resumes it.
#[test]
fn s21_2_c_2() {
    conformance(&["s21_context"]);
}
/// §21.2 Contract: **`makecontext(&uc, fn, argc, ...)`** modifies a context so that resuming it calls `fn` with `argc` integer arguments on the stack in `uc.uc_stack`. When `fn` returns, it resumes `uc.uc_link`, or exit
#[test]
fn s21_2_c_3() {
    conformance(&["s21_context"]);
}
/// §21.2 Contract: **`swapcontext(&old, &new)`** saves into `old` and resumes `new`.
#[test]
fn s21_2_c_4() {
    conformance(&["s21_context"]);
}
/// §21.2 Inputs: Arguments to `makecontext` must be `int`-sized. ⚑ HIST: on 64-bit ABIs, passing pointers is technically non-portable, although glibc supports 64-bit values on x86_64.
#[test]
fn s21_2_i_0() {
    conformance(&["s21_context"]);
}
