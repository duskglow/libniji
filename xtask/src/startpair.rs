//! `cargo xtask startup-pair --base <tree> [--head <tree>] [--runs N] [--rows a,b]` (STATIC-LAYOUT-FLIP,
//! D-X.148, owner ruling #7061): the static start-up rows of `bench/c/cbench_startup.c`, base against head, judged
//! at one read-only layout.
//!
//! A static program keeps the user's default linker layout (ruling #7061, option d), so its read-only data after
//! the text is one PT_LOAD or two by the text's size (D-X.143: 7 of 8 text pads split `cbench_startup`); each
//! extra segment costs every start one more mapping in the kernel (about 7.5k kernel instructions on a Zen 1
//! host). A change that only moves the text across that boundary moves the row by the mapping, not by its own
//! cost. This command links the bench program of both trees (the same command line as `cargo xtask bench`'s
//! libniji build), counts their read-only PT_LOADs after the text, and when the counts differ links both again
//! with the smallest text pad (a never-run `.text` object of k pages right after the start object, k = 1..16)
//! that makes them agree. It measures the unpadded pair (reported) and, on a flip, the padded pair (judged),
//! interleaved, `--runs` rounds (default 5), with `CBENCH_COUNTERS=1`, and prints `LAYOUT-FLIP` with the counts
//! and the pad. A user-mode row is WORSE when the head's median is above the base's maximum; a row counting
//! kernel instructions too (`startup_kinstr_c`) when it is above the base's maximum by more than
//! `KERNEL_SLACK` of the base's median (kernel counts move run to run by up to about 1%). Exit 1 on a WORSE
//! judged row. The pad exists only in these programs (`target/niji/bench/pair/`), never in a product.
//! Dynamic rows are not padded: libc.so.6 has one read-only segment at every text size (D-X.143).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The C benchmark whose static start-up rows this compares.
const BENCH: &str = "cbench_startup";
/// The counted start-up rows (`bench/c/cbench_startup.c`): user mode, and `startup_kinstr_c` with kernel mode.
const ROWS: &str = "startup_instr_c,startup_instr_setloc,startup_instr_printf,startup_instr_wctype,startup_kinstr_c";
/// The text pad's unit (one page) and the most units tried: a full turn of the 64 KiB read-only group bound.
pub const PAD_STEP: u64 = 0x1000;
pub const PAD_STEPS: u64 = 16;
/// How far above the base's maximum a kernel-mode row's head median may lie (fraction of the base's median).
pub const KERNEL_SLACK: f64 = 0.01;

/// The number of read-only (`R`) PT_LOADs after the last executable one, from `readelf -lW` text.
pub fn read_only_loads(phdrs: &str) -> Result<usize, String> {
    // `LOAD off vaddr paddr filesz memsz flags align`, the flags as separate words (`R E`).
    let flags: Vec<String> = phdrs
        .lines()
        .filter_map(|l| {
            let w: Vec<&str> = l.split_whitespace().collect();
            (w.first() == Some(&"LOAD") && w.len() >= 8).then(|| w[6..w.len() - 1].concat())
        })
        .collect();
    let text = flags.iter().rposition(|f| f.contains('E')).ok_or("no executable PT_LOAD")?;
    Ok(flags[text + 1..].iter().filter(|f| *f == "R").count())
}

/// The smallest pad (bytes, a multiple of [`PAD_STEP`], 0 first) at which `counts(pad)` gives the base and the
/// head the same read-only PT_LOAD count, with that count; `None` when no pad up to `PAD_STEPS` pages does.
pub fn matching_pad(mut counts: impl FnMut(u64) -> Result<(usize, usize), String>) -> Result<Option<(u64, usize)>, String> {
    for k in 0..=PAD_STEPS {
        let (b, h) = counts(k * PAD_STEP)?;
        if b == h {
            return Ok(Some((k * PAD_STEP, b)));
        }
    }
    Ok(None)
}

/// One row's verdict: is the head WORSE than the base (values: the program's best count per round)?
pub fn worse(base: &[f64], head: &[f64], kernel: bool) -> bool {
    if base.is_empty() || head.is_empty() {
        return false;
    }
    let (bmed, _, bmax) = crate::bench::stats(base);
    let (hmed, _, _) = crate::bench::stats(head);
    let allow = if kernel { bmax + KERNEL_SLACK * bmed } else { bmax };
    hmed > allow
}

/// One measured pair: per row, the base's and the head's values over the rounds, and whether the row counts
/// kernel mode too (its `ctr` line has no user-instructions field).
#[derive(Default)]
pub struct Pair {
    pub rows: BTreeMap<String, (Vec<f64>, Vec<f64>, bool)>,
}

/// The rows the ruling judges: the padded pair on a flip, else the unpadded one. Returns the WORSE rows.
pub fn judged_worse(unpadded: &Pair, padded: Option<&Pair>) -> Vec<String> {
    let p = padded.unwrap_or(unpadded);
    p.rows.iter().filter(|(_, (b, h, k))| worse(b, h, *k)).map(|(r, _)| r.clone()).collect()
}

/// Links `tree`'s `bench/c/cbench_startup.c` against `tree`'s products as `cargo xtask bench` does
/// (`bench.rs` `c_build`, `Build::Niji`), with a never-run text pad of `pad` bytes right after the start object.
fn link(tree: &Path, pad: u64, exe: &Path) -> Result<(), String> {
    let dir = exe.parent().ok_or("no output directory")?;
    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let mut cmd = Command::new(cc);
    cmd.args(["-std=c17", "-O2", "-fno-builtin", "-static", "-nostdlib"]).arg(tree.join("target/niji/niji-crt1.o"));
    if pad > 0 {
        let s = dir.join(format!("textpad-{pad:#x}.s"));
        fs::write(&s, format!("\t.text\n\t.skip {pad}, 0xcc\n")).map_err(|e| format!("{}: {e}", s.display()))?;
        cmd.arg(&s);
    }
    cmd.arg("-I").arg(tree.join("include")).arg(tree.join(format!("bench/c/{BENCH}.c"))).arg(tree.join("target/niji/libc.a")).arg("-lgcc");
    let out = cmd.arg("-o").arg(exe).output().map_err(|e| format!("{BENCH}: {e}"))?;
    if !out.status.success() {
        return Err(format!("{BENCH} ({}) does not link:\n{}", tree.display(), String::from_utf8_lossy(&out.stderr)));
    }
    Ok(())
}

fn ro_count(exe: &Path) -> Result<usize, String> {
    let out = Command::new("readelf").arg("-lW").arg(exe).output().map_err(|e| format!("readelf: {e}"))?;
    read_only_loads(&String::from_utf8_lossy(&out.stdout)).map_err(|e| format!("{}: {e}", exe.display()))
}

/// `runs` interleaved rounds of both programs (the order alternates per round).
fn measure(base: &Path, head: &Path, rows: &str, runs: usize) -> Result<Pair, String> {
    let mut p = Pair::default();
    for r in 0..runs {
        let order = if r % 2 == 0 { [(base, 0), (head, 1)] } else { [(head, 1), (base, 0)] };
        for (exe, side) in order {
            let out = crate::bench::c_command(BENCH, exe).env("CBENCH_COUNTERS", "1").env("CBENCH_ROWS", rows).output().map_err(|e| format!("{}: {e}", exe.display()))?;
            if !out.status.success() {
                return Err(format!("{} (counted run) failed", exe.display()));
            }
            for (row, v) in crate::bench::parse_counters(&out.stderr) {
                if v[0] <= 0.0 {
                    continue;
                }
                let e = p.rows.entry(row).or_insert_with(|| (Vec::new(), Vec::new(), v[4] < 0.0));
                if side == 0 { e.0.push(v[0]) } else { e.1.push(v[0]) }
            }
        }
    }
    if p.rows.is_empty() {
        return Err(format!("no counted row (CBENCH_COUNTERS=1 printed no `ctr` line for {rows}; perf_event_paranoid?)"));
    }
    Ok(p)
}

fn print_pair(label: &str, p: &Pair) {
    println!("{label}:");
    println!("| row | base min / median / max | head min / median / max | head - base (median) | verdict |");
    println!("|---|---|---|---|---|");
    for (row, (b, h, k)) in &p.rows {
        let (bm, bl, bh) = crate::bench::stats(b);
        let (hm, hl, hh) = crate::bench::stats(h);
        let v = if worse(b, h, *k) { "WORSE" } else { "ok" };
        println!("| {row}{} | {bl:.0} / {bm:.0} / {bh:.0} | {hl:.0} / {hm:.0} / {hh:.0} | {:+.0} | {v} |", if *k { " (user+kernel)" } else { "" }, hm - bm);
    }
}

pub fn run(root: &Path, args: &[String]) -> Result<(), String> {
    let (mut base, mut head, mut runs, mut rows, mut head_pad) = (None::<PathBuf>, root.to_path_buf(), 5usize, ROWS.to_string(), 0u64);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--base" => base = Some(PathBuf::from(it.next().ok_or("--base needs a libniji tree with its products built (`cargo xtask so`)")?)),
            "--head" => head = PathBuf::from(it.next().ok_or("--head needs a libniji tree")?),
            "--runs" => runs = it.next().and_then(|v| v.parse().ok()).filter(|&n| n > 0).ok_or("--runs needs a positive number")?,
            "--rows" => rows = it.next().filter(|v| !v.is_empty()).ok_or("--rows needs a comma list of cbench_startup rows")?.clone(),
            // Self-check: a fixed text pad on the head's program only (on top of the matching pad), the shape of a
            // change that does nothing but grow the text; with `--head` = `--base` the judged pair must be ok.
            "--head-pad" => head_pad = it.next().and_then(|v| v.parse().ok()).ok_or("--head-pad needs a byte count")?,
            other => return Err(format!("startup-pair: unknown argument `{other}` (usage: cargo xtask startup-pair --base <tree> [--head <tree>] [--runs N] [--rows a,b] [--head-pad BYTES])")),
        }
    }
    let base = base.ok_or("startup-pair: --base <tree> is required (a libniji tree with its products built)")?;
    for t in [&base, &head] {
        for f in ["target/niji/libc.a", "target/niji/niji-crt1.o"] {
            if !t.join(f).exists() {
                return Err(format!("startup-pair: {} has no {f} (run `cargo xtask so` there)", t.display()));
            }
        }
    }
    let out = root.join("target/niji/bench/pair");
    let exe = |side: &str, pad: u64| out.join(format!("{side}-pad{pad:#x}")).join(BENCH);
    let mut counts = |pad: u64| -> Result<(usize, usize), String> {
        let (b, h) = (exe("base", pad), exe("head", pad));
        link(&base, pad, &b)?;
        link(&head, pad + head_pad, &h)?;
        Ok((ro_count(&b)?, ro_count(&h)?))
    };
    let (b0, h0) = counts(0)?;
    println!("startup-pair: read-only PT_LOADs after the text, unpadded: base {b0}, head {h0}");
    let pad = if b0 == h0 {
        None
    } else {
        let (pad, n) = matching_pad(&mut counts)?.ok_or(format!("startup-pair: LAYOUT-FLIP base {b0} head {h0}: no text pad up to {PAD_STEPS} pages gives both the same count"))?;
        println!("startup-pair: LAYOUT-FLIP base {b0} head {h0}: judged at text pad {pad:#x} (both {n})");
        Some(pad)
    };
    if head_pad > 0 {
        println!("startup-pair: self-check: the head's program carries {head_pad} more text bytes at every pad (--head-pad)");
    }
    let unpadded = measure(&exe("base", 0), &exe("head", 0), &rows, runs)?;
    print_pair(&format!("unpadded pair (base {b0}, head {h0} read-only PT_LOADs){}", if pad.is_some() { ", reported" } else { ", judged" }), &unpadded);
    let padded = match pad {
        Some(p) => {
            let m = measure(&exe("base", p), &exe("head", p), &rows, runs)?;
            print_pair(&format!("padded pair (text pad {p:#x}), judged (ruling #7061)"), &m);
            Some(m)
        }
        None => None,
    };
    let bad = judged_worse(&unpadded, padded.as_ref());
    if bad.is_empty() {
        println!("startup-pair: ok ({runs} rounds per side)");
        Ok(())
    } else {
        Err(format!("startup-pair: WORSE on the judged pair: {}", bad.join(", ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPLIT: &str = "Program Headers:\n  Type Offset VirtAddr PhysAddr FileSiz MemSiz Flg Align\n  LOAD 0x000000 0x0000000000400000 0x0000000000400000 0x000270 0x000270 R   0x1000\n  LOAD 0x001000 0x0000000000401000 0x0000000000401000 0x05e2d1 0x05e2d1 R E 0x1000\n  LOAD 0x060000 0x0000000000460000 0x0000000000460000 0x00f000 0x00f000 R   0x1000\n  LOAD 0x070000 0x0000000000470000 0x0000000000470000 0x013000 0x013000 R   0x1000\n  LOAD 0x083e50 0x0000000000484e50 0x0000000000484e50 0x0031d8 0x00a2f8 RW  0x1000\n  TLS  0x083e50 0x0000000000484e50 0x0000000000484e50 0x000010 0x000058 R   0x8\n";

    // no spec clause (bench tooling, D-X.148): the read-only segments are counted after the text only, and a
    // non-LOAD header with the same flags (TLS) is not one.
    #[test]
    fn read_only_loads_counts_only_after_the_text() {
        assert_eq!(read_only_loads(SPLIT).unwrap(), 2);
        let one = SPLIT.replace("  LOAD 0x070000 0x0000000000470000 0x0000000000470000 0x013000 0x013000 R   0x1000\n", "");
        assert_eq!(read_only_loads(&one).unwrap(), 1);
        assert!(read_only_loads("  LOAD 0x0 0x0 0x0 0x10 0x10 R 0x1000\n").is_err());
    }

    // no spec clause (D-X.148, ruling #7061): the pad is the smallest one at which both counts agree, 0 when they
    // already do; none in range is reported, never a mismatched pad.
    #[test]
    fn matching_pad_is_the_smallest_pad_with_equal_counts() {
        // head splits until its text crosses the next boundary at 3 pages; base splits until 5.
        let at = |b_until: u64, h_until: u64| move |pad: u64| Ok((if pad < b_until * PAD_STEP { 1 } else { 2 }, if pad < h_until * PAD_STEP { 2 } else { 1 }));
        assert_eq!(matching_pad(at(3, 5)).unwrap(), Some((3 * PAD_STEP, 2)));
        let mut seen = Vec::new();
        let r = matching_pad(|pad| {
            seen.push(pad);
            Ok(if pad < 3 * PAD_STEP { (1, 2) } else { (2, 2) })
        })
        .unwrap();
        assert_eq!(r, Some((3 * PAD_STEP, 2)));
        assert_eq!(seen, vec![0, PAD_STEP, 2 * PAD_STEP, 3 * PAD_STEP]);
        assert_eq!(matching_pad(|_| Ok((1, 1))).unwrap(), Some((0, 1)));
        assert_eq!(matching_pad(|_| Ok((1, 2))).unwrap(), None);
    }

    fn pair(rows: &[(&str, &[f64], &[f64], bool)]) -> Pair {
        Pair { rows: rows.iter().map(|(r, b, h, k)| (r.to_string(), (b.to_vec(), h.to_vec(), *k))).collect() }
    }

    // no spec clause (D-X.148, ruling #7061): a change that only flips the read-only segment count passes (judged at
    // the matched pad), and a real cost at the matched pad still fails, user or kernel mode.
    #[test]
    fn a_flip_alone_passes_and_a_real_cost_at_the_matched_layout_fails() {
        let base_k = [165_900.0, 166_100.0, 166_400.0, 165_800.0, 166_000.0];
        let flip_k: Vec<f64> = base_k.iter().map(|v| v + 7_500.0).collect();
        let user = [2_151.0; 5];
        let unpadded = pair(&[("startup_kinstr_c", &base_k, &flip_k, true), ("startup_instr_c", &user, &user, false)]);
        assert_eq!(judged_worse(&unpadded, None), vec!["startup_kinstr_c".to_string()]);
        let matched = pair(&[("startup_kinstr_c", &base_k, &base_k, true), ("startup_instr_c", &user, &user, false)]);
        assert!(judged_worse(&unpadded, Some(&matched)).is_empty(), "a flip alone must pass at the matched pad");
        let cost_k: Vec<f64> = base_k.iter().map(|v| v + 3_000.0).collect();
        let cost = pair(&[("startup_kinstr_c", &base_k, &cost_k, true), ("startup_instr_c", &user, &[2_157.0; 5], false)]);
        assert_eq!(judged_worse(&unpadded, Some(&cost)), vec!["startup_instr_c".to_string(), "startup_kinstr_c".to_string()]);
        // kernel noise within the slack is not a cost; one user instruction above the base's maximum is.
        let noise: Vec<f64> = base_k.iter().map(|v| v + 1_000.0).collect();
        assert!(!worse(&base_k, &noise, true) && worse(&user, &[2_152.0; 5], false));
    }
}
