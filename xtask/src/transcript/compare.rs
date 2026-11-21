//! The comparator: two outcomes of one transcript (reference and libniji, or the record and a live run), compared
//! aspect by aspect: exit status, stdout, stderr (each stream on its own, so text on the wrong stream is a mismatch),
//! and every path written. Each difference is one [`Mismatch`] with a readable diff.

use super::format::{Outcome, Written, escape};
use std::collections::BTreeMap;

/// One difference between two outcomes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Mismatch {
    /// `status`, `stdout`, `stderr` or `written <path>`.
    pub what: String,
    /// The diff or the two values, ready to print.
    pub detail: String,
}

impl std::fmt::Display for Mismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MISMATCH {}: {}", self.what, self.detail)
    }
}

/// The sha256 of `bytes` (hex), from `sha256sum` (coreutils, on every host).
pub(crate) fn sha256(bytes: &[u8]) -> String {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let run = || -> Option<String> {
        let mut c = Command::new("sha256sum").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().ok()?;
        let mut stdin = c.stdin.take()?;
        let data = bytes.to_vec();
        let feeder = std::thread::spawn(move || stdin.write_all(&data));
        let o = c.wait_with_output().ok()?;
        feeder.join().ok()?.ok()?;
        String::from_utf8_lossy(&o.stdout).get(..64).map(str::to_string)
    };
    run().unwrap_or_else(|| "sha256sum-failed".into())
}

/// Compare outcome `b` against `a`. The labels name the sides in the details (`reference`, `niji`, `record`).
pub(crate) fn compare(a_label: &str, a: &Outcome, b_label: &str, b: &Outcome) -> Vec<Mismatch> {
    let mut out = Vec::new();
    if a.status != b.status {
        out.push(Mismatch { what: "status".into(), detail: format!("{a_label} {}, {b_label} {}", a.status, b.status) });
    }
    for (name, x, y, x_other, y_other) in [("stdout", &a.stdout, &b.stdout, &a.stderr, &b.stderr), ("stderr", &a.stderr, &b.stderr, &a.stdout, &b.stdout)] {
        if x == y {
            continue;
        }
        let mut detail = format!("(- {a_label}, + {b_label})\n{}", diff(x, y));
        let other = if name == "stdout" { "stderr" } else { "stdout" };
        if !x.is_empty() && x == y_other {
            detail.push_str(&format!("\n  note: {b_label} wrote {a_label}'s {name} text to {other}"));
        } else if !y.is_empty() && y == x_other {
            detail.push_str(&format!("\n  note: {b_label} wrote to {name} the text {a_label} wrote to {other}"));
        }
        out.push(Mismatch { what: name.into(), detail });
    }
    let am: BTreeMap<&[u8], &Written> = a.written.iter().map(|(p, w)| (p.as_slice(), w)).collect();
    let bm: BTreeMap<&[u8], &Written> = b.written.iter().map(|(p, w)| (p.as_slice(), w)).collect();
    let mut paths: Vec<&[u8]> = am.keys().chain(bm.keys()).copied().collect();
    paths.sort_unstable();
    paths.dedup();
    for p in paths {
        let what = format!("written {}", escape(p, true));
        let detail = match (am.get(p), bm.get(p)) {
            (Some(x), Some(y)) => written_diff(a_label, x, b_label, y),
            (Some(x), None) => Some(format!("{a_label}: {}; {b_label}: not written", kind(x))),
            (None, Some(y)) => Some(format!("{a_label}: not written; {b_label}: {}", kind(y))),
            (None, None) => None,
        };
        if let Some(detail) = detail {
            out.push(Mismatch { what, detail });
        }
    }
    out
}

fn kind(w: &Written) -> String {
    match w {
        Written::File(b) => format!("a file of {} bytes", b.len()),
        Written::Digest(_, n) => format!("a file of {n} bytes"),
        Written::Dir => "a directory".into(),
        Written::Link(t) => format!("a link to {}", escape(t, false)),
        Written::Other(m) => format!("a special file (mode {m:o})"),
        Written::Removed => "removed".into(),
    }
}

/// The difference between two written items, or `None` when they are the same. A file recorded as a digest equals a
/// file whose bytes have that digest and size.
fn written_diff(a_label: &str, x: &Written, b_label: &str, y: &Written) -> Option<String> {
    let digest = |b: &[u8]| (sha256(b), b.len() as u64);
    match (x, y) {
        (Written::File(p), Written::File(q)) if p == q => None,
        (Written::File(p), Written::File(q)) => {
            let text = |b: &[u8]| b.iter().all(|&c| c == b'\t' || c == b'\n' || (0x20..0x7f).contains(&c));
            if text(p) && text(q) {
                Some(format!("contents differ (- {a_label}, + {b_label})\n{}", diff(p, q)))
            } else {
                let (hp, np) = digest(p);
                let (hq, nq) = digest(q);
                Some(format!("contents differ: {a_label} {np} bytes sha256 {hp}, {b_label} {nq} bytes sha256 {hq}"))
            }
        }
        (Written::Digest(h, n), Written::File(q)) | (Written::File(q), Written::Digest(h, n)) => {
            let (hq, nq) = digest(q);
            if (&hq, nq) == (h, *n) {
                None
            } else {
                let (from, to) = if matches!(x, Written::Digest(..)) { ((h.clone(), *n), (hq, nq)) } else { ((hq, nq), (h.clone(), *n)) };
                Some(format!("contents differ: {a_label} {} bytes sha256 {}, {b_label} {} bytes sha256 {}", from.1, from.0, to.1, to.0))
            }
        }
        _ if x == y => None,
        _ => Some(format!("{a_label}: {}; {b_label}: {}", kind(x), kind(y))),
    }
}

/// The lines shown around each change, and the most diff lines printed.
const CONTEXT: usize = 2;
const DIFF_MAX: usize = 60;

/// A line diff of `a` and `b` (`-` only in `a`, `+` only in `b`, ` ` context), escaped so tabs, trailing spaces and
/// control bytes are visible, and a last line without a newline is marked.
pub(crate) fn diff(a: &[u8], b: &[u8]) -> String {
    let split = |x: &[u8]| -> Vec<String> {
        if x.is_empty() {
            return Vec::new();
        }
        let open = !x.ends_with(b"\n");
        let body = x.strip_suffix(b"\n").unwrap_or(x);
        let mut v: Vec<String> = body.split(|&c| c == b'\n').map(|l| escape(l, false)).collect();
        if open {
            if let Some(last) = v.last_mut() {
                last.push_str("  (no newline at end)");
            }
        }
        v
    };
    let (la, lb) = (split(a), split(b));
    let ops = line_ops(&la, &lb);
    // Keep the lines within CONTEXT of a change.
    let changed: Vec<bool> = ops.iter().map(|(t, _)| *t != ' ').collect();
    let mut out = Vec::new();
    let mut skipped = false;
    for (i, (tag, line)) in ops.iter().enumerate() {
        let lo = i.saturating_sub(CONTEXT);
        let hi = (i + CONTEXT + 1).min(ops.len());
        if changed[lo..hi].iter().any(|&c| c) {
            if skipped {
                out.push("  ...".to_string());
                skipped = false;
            }
            out.push(format!("  {tag}{line}"));
        } else {
            skipped = true;
        }
    }
    if out.len() > DIFF_MAX {
        let more = out.len() - DIFF_MAX;
        out.truncate(DIFF_MAX);
        out.push(format!("  ... ({more} more diff lines)"));
    }
    out.join("\n")
}

/// The edit script of two line lists by longest common subsequence; above 4M cells, the common prefix and suffix are
/// kept and everything between is shown as removed and added.
fn line_ops(a: &[String], b: &[String]) -> Vec<(char, String)> {
    let pre = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let suf = a[pre..].iter().rev().zip(b[pre..].iter().rev()).take_while(|(x, y)| x == y).count();
    let (ma, mb) = (&a[pre..a.len() - suf], &b[pre..b.len() - suf]);
    let mut ops: Vec<(char, String)> = a[..pre].iter().map(|l| (' ', l.clone())).collect();
    if ma.len().saturating_mul(mb.len()) > 4 << 20 {
        ops.extend(ma.iter().map(|l| ('-', l.clone())));
        ops.extend(mb.iter().map(|l| ('+', l.clone())));
    } else {
        let (n, m) = (ma.len(), mb.len());
        let mut lcs = vec![0u32; (n + 1) * (m + 1)];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                lcs[i * (m + 1) + j] = if ma[i] == mb[j] { lcs[(i + 1) * (m + 1) + j + 1] + 1 } else { lcs[(i + 1) * (m + 1) + j].max(lcs[i * (m + 1) + j + 1]) };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < n || j < m {
            if i < n && j < m && ma[i] == mb[j] {
                ops.push((' ', ma[i].clone()));
                i += 1;
                j += 1;
            } else if j < m && (i == n || lcs[i * (m + 1) + j + 1] >= lcs[(i + 1) * (m + 1) + j]) {
                ops.push(('+', mb[j].clone()));
                j += 1;
            } else {
                ops.push(('-', ma[i].clone()));
                i += 1;
            }
        }
    }
    ops.extend(a[a.len() - suf..].iter().map(|l| (' ', l.clone())));
    ops
}

#[cfg(test)]
mod tests {
    use super::super::format::Status;
    use super::*;

    fn outcome() -> Outcome {
        Outcome {
            status: Status::Exit(0),
            stdout: b"2 libs found in cache `@ROOT@/etc/ld.so.cache'\n\tlibz.so.1 (libc6,x86-64) => /usr/lib/libz.so.1\n".to_vec(),
            stderr: Vec::new(),
            written: vec![(b"etc/out".to_vec(), Written::File(b"one\ntwo\n".to_vec()))],
        }
    }

    #[test]
    fn identical_outcomes_have_no_mismatch() {
        // Trace: no spec clause (test-harness tooling; UTIL-HARNESS comparator).
        assert_eq!(compare("reference", &outcome(), "niji", &outcome()), vec![]);
    }

    #[test]
    fn a_changed_byte_in_stdout_is_a_mismatch() {
        // Trace: no spec clause (test-harness tooling; UTIL-HARNESS comparator).
        let mut b = outcome();
        let at = b.stdout.iter().position(|&c| c == b'z').unwrap();
        b.stdout[at] = b'Z';
        let m = compare("reference", &outcome(), "niji", &b);
        assert_eq!(m.len(), 1, "{m:?}");
        assert_eq!(m[0].what, "stdout");
        assert!(m[0].detail.contains("  -\\tlibz.so.1") && m[0].detail.contains("  +\\tlibZ.so.1"), "{}", m[0].detail);
        assert!(m[0].detail.contains("   2 libs found"), "context line: {}", m[0].detail);
    }

    #[test]
    fn a_changed_exit_status_is_a_mismatch() {
        // Trace: no spec clause (test-harness tooling; UTIL-HARNESS comparator).
        for status in [Status::Exit(1), Status::Signal(0)] {
            let mut b = outcome();
            b.status = status;
            let m = compare("reference", &outcome(), "niji", &b);
            assert_eq!(m.len(), 1, "{m:?}");
            assert_eq!(m[0].what, "status");
            assert_eq!(m[0].detail, format!("reference exit 0, niji {status}"));
        }
    }

    #[test]
    fn the_same_text_on_the_other_stream_is_a_mismatch() {
        // Trace: no spec clause (test-harness tooling; UTIL-HARNESS comparator).
        let mut b = outcome();
        b.stderr = std::mem::take(&mut b.stdout);
        let m = compare("reference", &outcome(), "niji", &b);
        let what: Vec<&str> = m.iter().map(|x| x.what.as_str()).collect();
        assert_eq!(what, ["stdout", "stderr"], "{m:?}");
        assert!(m[0].detail.contains("note: niji wrote reference's stdout text to stderr"), "{}", m[0].detail);
    }

    #[test]
    fn a_written_file_that_differs_is_a_mismatch() {
        // Trace: no spec clause (test-harness tooling; UTIL-HARNESS comparator).
        let mut b = outcome();
        b.written[0].1 = Written::File(b"one\nTwo\n".to_vec());
        let m = compare("reference", &outcome(), "niji", &b);
        assert_eq!(m.len(), 1, "{m:?}");
        assert_eq!(m[0].what, "written etc/out");
        assert!(m[0].detail.contains("  -two") && m[0].detail.contains("  +Two"), "{}", m[0].detail);
        // A binary file, a digest that does not match, a missing file and a different kind are mismatches too.
        let bin = |x: &[u8]| vec![(b"etc/out".to_vec(), Written::File(x.to_vec()))];
        let (mut x, mut y) = (outcome(), outcome());
        x.written = bin(&[0, 1]);
        y.written = bin(&[0, 2]);
        assert!(compare("a", &x, "b", &y)[0].detail.contains("contents differ: a 2 bytes sha256"));
        y.written = vec![(b"etc/out".to_vec(), Written::Digest(sha256(&[0, 1]), 2))];
        assert_eq!(compare("a", &x, "b", &y), vec![], "a matching digest is the same file");
        y.written = vec![(b"etc/out".to_vec(), Written::Digest(sha256(&[0, 1]), 3))];
        assert_eq!(compare("a", &x, "b", &y).len(), 1, "a digest with another size");
        y.written = Vec::new();
        assert!(compare("a", &x, "b", &y)[0].detail.ends_with("b: not written"));
        y.written = vec![(b"etc/out".to_vec(), Written::Link(b"x".to_vec()))];
        assert!(compare("a", &x, "b", &y)[0].detail.contains("b: a link to x"));
    }

    #[test]
    fn the_diff_marks_a_missing_final_newline_and_elides_far_lines() {
        // Trace: no spec clause (test-harness tooling; UTIL-HARNESS comparator).
        let a: Vec<u8> = (0..20).flat_map(|i| format!("line {i}\n").into_bytes()).collect();
        let mut b = a.clone();
        b.pop();
        let d = diff(&a, &b);
        assert!(d.contains("  +line 19  (no newline at end)") && d.contains("  -line 19"), "{d}");
        assert!(d.starts_with("  ...\n") && !d.contains("line 3\n"), "{d}");
    }
}
