//! Charset names that name an installed charmap without being one of its own names (spec §6.4,
//! `spec/encoding/iconv-names.md`, rule 5).
//!
//! `iconv_open` matches a name exactly ([`super::names_charset`]), so a name the system knows only
//! from another source (`WINDOWS-1252` for the charmap `CP1252`, `KOI8R` for `KOI8-R`) needs a row
//! of its own. Each row is a name `iconv -l` lists on the reference system and the charmap (file
//! name, matching form) that holds the same conversion: a black-box probe of the reference
//! (`tools/probe/iconv_aliases.py`, fingerprints by `tools/probe/iconv_fingerprint.c`) decodes every
//! one- and two-byte input and encodes every code point (all of the BMP, a stride of the rest) the
//! same way through the name and through one of the charmap's names. The rows are data,
//! `spec/encoding/iconv-aliases.tsv`, read here at compile time. The table does not depend on which
//! charmap files are installed: a name whose charmap is missing names nothing, as the charmap's own
//! name does. Where a generated charmap carries a row's name, the row points to that charmap:
//! `HPTHAI8` is `ISO-8859-11`, which the probe found identical to `HP-THAI8`.
//!
//! Sorted by name for a binary search; the core test `s6_4_c_1_charmap_aliases` checks the order,
//! that no row is a built-in name, and that the generated charmaps (`spec/encoding/charmaps.tsv`)
//! never give a row's name to another charmap.
//!
//! The library holds the table without pointers ([`TEXT`], the data file's bytes, and [`ROWS`],
//! offsets into it computed at compile time): a table of slices in a shared library is 32 bytes of
//! relocated data per row, which the loader writes at every process start whether or not the
//! process ever calls `iconv_open` (106 rows as slices were one more page of libc.so.6 written and
//! one more page fault at every start of a dynamic program, Skylake-SP host). Bytes and offsets need
//! no relocation and stay in read-only data that start-up never reads.

/// The data file: `#` comment lines, then one row per line, `name<TAB>charmap<LF>`.
static TEXT: [u8; TEXT_LEN] = TEXT_BYTES;

/// The data file's length.
const TEXT_LEN: usize = include_bytes!("../../../../spec/encoding/iconv-aliases.tsv").len();

/// The number of rows.
const ROW_COUNT: usize = row_count();

/// Per row, offsets in [`TEXT`]: where its name starts, where it ends (the tab; the charmap starts
/// one byte later) and where the charmap ends (the line feed).
static ROWS: [[u16; 3]; ROW_COUNT] = rows();

/// Whether a line starting at `at` is a row (not a comment or empty).
#[allow(clippy::indexing_slicing)]
const fn is_row(t: &[u8], at: usize) -> bool {
    t[at] != b'#' && t[at] != b'\n'
}

// A const fn: a missing final line feed, or a row without a tab, fails the build.
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
const fn row_count() -> usize {
    let t = &TEXT_BYTES;
    assert!(t[t.len() - 1] == b'\n', "iconv-aliases.tsv must end in a line feed");
    let (mut n, mut at) = (0, 0);
    while at < t.len() {
        let row = is_row(t, at);
        let mut tabs = 0;
        while t[at] != b'\n' {
            tabs += usize::from(t[at] == b'\t');
            at += 1;
        }
        assert!(!row || tabs == 1, "an iconv-aliases.tsv row is name<TAB>charmap");
        n += usize::from(row);
        at += 1;
    }
    n
}

/// [`TEXT`] for the const fns (a `static` cannot be read in constant evaluation).
const TEXT_BYTES: [u8; TEXT_LEN] = *include_bytes!("../../../../spec/encoding/iconv-aliases.tsv");

// A const fn: a file past `u16::MAX` bytes fails the build.
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::as_conversions)]
const fn rows() -> [[u16; 3]; ROW_COUNT] {
    assert!(TEXT_LEN <= u16::MAX as usize, "the alias file outgrew u16 offsets");
    let t = &TEXT_BYTES;
    let mut r = [[0u16; 3]; ROW_COUNT];
    let (mut i, mut at) = (0, 0);
    while at < t.len() {
        let start = at;
        let mut tab = at;
        while t[at] != b'\n' {
            if t[at] == b'\t' {
                tab = at;
            }
            at += 1;
        }
        if is_row(t, start) {
            r[i] = [start as u16, tab as u16, at as u16];
            i += 1;
        }
        at += 1;
    }
    r
}

/// The bytes of [`TEXT`] from `from` to `to`.
fn span(from: u16, to: u16) -> &'static [u8] {
    TEXT.get(usize::from(from)..usize::from(to)).unwrap_or(&[])
}

/// A row's charmap: after its tab.
fn charmap(tab: u16, end: u16) -> &'static [u8] {
    span(tab.saturating_add(1), end)
}

/// The charmap `name` (matching form, [`super::Name::form`]) stands for when it is not a name of
/// that charmap itself, if any.
pub fn charmap_alias(name: &[u8]) -> Option<&'static [u8]> {
    let i = ROWS.binary_search_by(|&[n, t, _]| span(n, t).cmp(name)).ok()?;
    ROWS.get(i).map(|&[_, t, e]| charmap(t, e))
}

/// Every row, in order: (name, charmap).
pub fn charmap_aliases() -> impl Iterator<Item = (&'static [u8], &'static [u8])> {
    ROWS.iter().map(|&[n, t, e]| (span(n, t), charmap(t, e)))
}
