//! The generated collation root (`niji_collate_root`, D-31.32) against the reference's en_US.UTF-8.
//! A child module of `reference.rs` (its helpers: `reference`, `leak`, `diff_collate`, the generated
//! charmaps), kept in its own file so cases added to either file do not collide.

use std::path::Path;

use niji::locale::file::Category;
use niji_locale::{Files, Kind, TryVec};

use super::{diff_collate, leak, reference, Generated};

/// The root locale source of `niji_collate_root` as the locale file, with the generated charmaps.
struct Root(Vec<u8>);

impl Files for Root {
    fn read(&mut self, kind: Kind, name: &[u8]) -> niji_locale::Result<TryVec<u8>> {
        match kind {
            Kind::Locale => TryVec::from_slice(&self.0).map_err(Into::into),
            Kind::Charmap => Generated.read(kind, name),
        }
    }
}

/// no spec clause (generated locale data by probe: owner ruling #7791, decision #7958). The
/// generated root collation (`niji_collate_root`: the DUCET and spec/locale/collate-root.txt)
/// compiled by libniji's localedef orders as the reference's en_US.UTF-8 does (the root order: 206
/// of the reference's 327 UTF-8 locales order as en_US; results/locale-collate-2026-11-23.md).
/// Every code point U+0001-U+10FFFF but the surrogates, in the reference's `strxfrm` order: for
/// each adjacent pair the order and the level at which the two differ (the filler test of
/// tools/probe/collate/collorder.c) must be the same; then the corpus and `wcscoll`
/// as [`super::diff_collate`] compares them (bracket ranges aside: see the comment there).
#[test]
fn generated_root_collation_is_the_references_en_us() {
    use niji::locale::collate::{compare, key_bytes, Collate, Collation};
    use std::cmp::Ordering;
    let Some(dir) = reference("en_US", "UTF-8") else { return };
    let theirs = std::fs::read(dir.join("out").join("LC_COLLATE")).expect("the reference's LC_COLLATE");
    let _ = std::fs::remove_dir_all(&dir);
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let src = niji_collate_root::source(&repo).expect("the root source");
    let out = niji_locale::compile_output(b"root", b"UTF-8", &mut Root(src.into_bytes())).expect("compiling the root");
    assert!(out.errors.is_empty(), "the root source: {:?}", out.errors.first());
    let ours = out.files[3].as_ref().expect("the root's LC_COLLATE").as_slice().to_vec();
    let cat = |b: &Vec<u8>| Category::parse(leak(b.clone())).expect("a category header");
    let (Some(Collation::Rules(t)), Some(Collation::Rules(o))) = (Collate::parse(cat(&theirs)), Collate::parse(cat(&ours))) else {
        panic!("both collations have rules");
    };
    let key = |s: &[u8]| {
        let mut k = vec![0u8; 4096];
        let n = key_bytes(&t, s, &mut k).unwrap_or(0);
        k.truncate(n);
        k
    };
    let strs: Vec<Vec<u8>> = (1u32..0x11_0000).filter_map(char::from_u32).map(|c| c.to_string().into_bytes()).collect();
    let mut sorted: Vec<(Vec<u8>, usize)> = strs.iter().enumerate().map(|(i, s)| (key(s), i)).collect();
    sorted.sort();
    // 0: equal; 1-3: the first level the fillers show; 4: beyond level 3; 9: the other way round.
    let level = |c: &Collate, a: &[u8], b: &[u8]| -> u8 {
        match compare(c, a, b) {
            Ordering::Equal => return 0,
            Ordering::Greater => return 9,
            Ordering::Less => {}
        }
        for (l, (f1, f2)) in [("a", "b"), ("a", "\u{e1}"), ("a", "A")].into_iter().enumerate() {
            if compare(c, [a, f2.as_bytes()].concat().as_slice(), [b, f1.as_bytes()].concat().as_slice()) == Ordering::Less {
                return l as u8 + 1;
            }
        }
        4
    };
    let mut diffs = Vec::new();
    let mut count = 0usize;
    for w in sorted.windows(2) {
        let (a, b) = (&strs[w[0].1], &strs[w[1].1]);
        let (lt, lo) = (level(&t, a, b), level(&o, a, b));
        if lt != lo {
            count += 1;
            if diffs.len() < 12 {
                diffs.push(format!("{:?} {:?}: ref level {lt}, ours {lo}", String::from_utf8_lossy(a), String::from_utf8_lossy(b)));
            }
        }
    }
    println!("{} code points compared, {count} adjacent pairs differ", strs.len());
    let mut corpus = Vec::new();
    diff_collate(cat(&theirs), cat(&ours), true, &mut corpus);
    // Bracket-range order (collation sequence numbers) is not part of the probed order: the
    // reference's differs from its strcoll order (U+03B1 before U+0041). Open, not asserted here
    // (handoff/LOCALE-COLLATE-TAILORINGS.md).
    let (collseq, corpus): (Vec<String>, Vec<String>) = corpus.into_iter().partition(|d| d.starts_with("LC_COLLATE collseq"));
    println!("collseq differences (open): {}", collseq.len());
    assert!(count == 0 && corpus.is_empty(), "{count} pairs differ from the reference:\n{}\ncorpus:\n{}", diffs.join("\n"), corpus.join("\n"));
}
