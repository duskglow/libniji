//! The `STB_GNU_UNIQUE` table `dlopen` keeps (`liella_ld::reloc::GrowTable`, DL-LIBSTDCXX): no limit
//! on entries or name bytes but memory, a rollback to a mark, a failed record that leaves the table
//! as it was, and a diagnostic for that failure that names what ran out (never "malformed": the
//! object's tables are sound). The loader backs the table with its own mappings; here a `Vec`, and
//! one capped at a known size stands in for memory running out.
//!
//! Trace: s27_1_g_0 (one definition per unique name for the namespace); the out-of-memory
//! diagnostic: no spec clause (spec/security/threat-model.md rule 4, failures say what failed).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::as_conversions, clippy::cast_possible_truncation)]

use liella_ld::elf::Sym;
use liella_ld::reloc::{GrowTable, RelocError, Stack, UniqueEntry};
use liella_ld::symbols::{Def, Found, LookupError, UniqueRegistry, UNIQUE_NO_MEMORY};

/// A `Vec` that refuses to hold more than `cap` items.
struct Capped<T> {
    v: Vec<T>,
    cap: usize,
}

impl<T: Copy> Stack<T> for Capped<T> {
    fn items(&self) -> &[T] {
        &self.v
    }
    fn try_extend(&mut self, more: &[T]) -> bool {
        if self.v.len() + more.len() > self.cap {
            return false;
        }
        self.v.extend_from_slice(more);
        true
    }
    fn truncate(&mut self, n: usize) {
        self.v.truncate(n);
    }
}

fn found(obj: usize, k: u64) -> Found {
    let st = Sym { name: 0, info: 0x1a, other: 0, shndx: 1, value: k, size: 4 };
    Found { obj, def: Def { sym: k as u32, st, version: 1, hidden: false } }
}

fn version(k: usize) -> &'static [u8] {
    if k % 2 == 0 { b"" } else { b"GLIBCXX_3.4" }
}

/// s27_1_g_0: 5000 names (about 90 KiB of keys) are all kept, each under its own version only; a
/// rollback forgets exactly the entries after its mark.
#[test]
fn s27_1_g_0_unique_table_keeps_every_name_and_rolls_back() {
    let mut t: GrowTable<Vec<UniqueEntry>, Vec<u8>> = GrowTable::new(Vec::new(), Vec::new());
    let names: Vec<Vec<u8>> = (0..5000).map(|k| format!("_ZN11unique_name{k}E").into_bytes()).collect();
    for (k, n) in names.iter().enumerate() {
        assert_eq!(t.find(n, version(k)), None, "entry {k} before it is recorded");
        t.record(n, version(k), found(k % 7, k as u64)).unwrap();
    }
    assert_eq!(t.len(), 5000);
    for (k, n) in names.iter().enumerate() {
        assert_eq!(t.find(n, version(k)), Some(found(k % 7, k as u64)), "entry {k}");
        assert_eq!(t.find(n, version(k + 1)), None, "entry {k} under the other version");
    }
    t.truncate(1000);
    assert_eq!(t.len(), 1000);
    assert_eq!(t.find(&names[999], version(999)), Some(found(999 % 7, 999)), "an entry before the mark stays");
    assert_eq!(t.find(&names[1000], version(1000)), None, "an entry after the mark is gone");
    t.record(&names[1000], version(1000), found(3, 77)).unwrap();
    assert_eq!(t.find(&names[1000], version(1000)), Some(found(3, 77)), "a name after the mark binds anew");
    assert_eq!(t.owners(999).collect::<Vec<_>>(), [999 % 7, 3]);
}

/// no spec clause: when either stack cannot grow, `record` answers `NoMemory`, the table keeps only
/// what it had (a later record that fits still succeeds), and the text says memory ran out.
#[test]
fn unique_table_out_of_memory_keeps_the_table_and_says_what_ran_out() {
    // Key bytes run out: 6 of 10 used, then a name that fits with a version that does not.
    let mut t = GrowTable::new(Capped { v: Vec::new(), cap: 100 }, Capped { v: Vec::new(), cap: 10 });
    t.record(b"abcdef", b"", found(1, 1)).unwrap();
    assert_eq!(t.record(b"ghij", b"V1", found(2, 2)), Err(LookupError::NoMemory));
    assert_eq!((t.len(), t.find(b"ghij", b"V1"), t.find(b"ghij", b"")), (1, None, None), "the failed key left no entry");
    t.record(b"xyz", b"", found(3, 3)).expect("the failed key's bytes were taken back: 9 of 10 fit");
    assert_eq!(t.find(b"abcdef", b""), Some(found(1, 1)));
    // Entries run out: two fit, the third fails and leaves no key bytes behind.
    let mut t = GrowTable::new(Capped { v: Vec::new(), cap: 2 }, Capped { v: Vec::new(), cap: 100 });
    t.record(b"a", b"", found(1, 1)).unwrap();
    t.record(b"b", b"", found(1, 2)).unwrap();
    assert_eq!(t.record(b"c", b"", found(1, 3)), Err(LookupError::NoMemory));
    assert_eq!((t.len(), t.find(b"c", b""), t.find(b"b", b"")), (2, None, Some(found(1, 2))));
    t.truncate(1);
    t.record(b"c", b"", found(1, 3)).expect("after a rollback the freed entry is usable");
    // A rollback hands back the key bytes of the entries it forgets: 3 of 3 used, cut to 1, then 2 more fit.
    let mut t = GrowTable::new(Capped { v: Vec::new(), cap: 10 }, Capped { v: Vec::new(), cap: 3 });
    for n in [b"a", b"b", b"c"] {
        t.record(n, b"", found(1, 1)).unwrap();
    }
    t.truncate(1);
    t.record(b"d", b"", found(1, 4)).expect("a rollback returns its key bytes");
    t.record(b"e", b"", found(1, 5)).expect("a rollback returns its key bytes");
    assert_eq!((t.find(b"a", b""), t.find(b"b", b""), t.find(b"e", b"")), (Some(found(1, 1)), None, Some(found(1, 5))));
    // The text a failed relocation or dlsym prints after `<object>: `.
    let text = RelocError::Lookup(LookupError::NoMemory).message();
    assert_eq!(text, UNIQUE_NO_MEMORY);
    let s = String::from_utf8_lossy(text);
    assert!(s.contains("cannot allocate memory") && s.contains("unique symbol table") && !s.contains("malformed"), "{s}");
    assert_eq!(RelocError::Lookup(LookupError::NoSymtab).message(), b"malformed symbol tables\n", "a real table fault still says so");
}
