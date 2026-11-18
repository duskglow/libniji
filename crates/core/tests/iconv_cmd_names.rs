//! The `iconv` command's list of built-in charset names (`csys/util/iconv-names.h`, UTIL-7) is the
//! library's table: `iconv -l` lists exactly the names `iconv_open` accepts.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, missing_docs)]

use std::collections::BTreeSet;

/// The quoted names of the C header, in order.
fn header_names() -> Vec<Vec<u8>> {
    let text = include_str!("../../../csys/util/iconv-names.h");
    let body = text.split_once('{').unwrap().1.split_once("};").unwrap().0;
    body.split('"').skip(1).step_by(2).map(|s| s.as_bytes().to_vec()).collect()
}

#[test]
fn iconv_listing_names_match_the_builtins() {
    // Trace: s31_6_g_3 (`-l` lists the names the command accepts: exactly the names libniji converts).
    let header = header_names();
    let lib: Vec<Vec<u8>> = niji::iconv::builtin_names().map(<[u8]>::to_vec).collect();
    let (h, l): (BTreeSet<_>, BTreeSet<_>) = (header.iter().cloned().collect(), lib.iter().cloned().collect());
    assert_eq!(header.len(), h.len(), "iconv-names.h repeats a name");
    let missing: Vec<_> = l.difference(&h).map(|n| String::from_utf8_lossy(n).into_owned()).collect();
    let extra: Vec<_> = h.difference(&l).map(|n| String::from_utf8_lossy(n).into_owned()).collect();
    assert!(missing.is_empty() && extra.is_empty(), "iconv-names.h lacks {missing:?}, has unknown {extra:?}");
    for n in &header {
        assert!(niji::iconv::builtin(n).is_some(), "{} is not a built-in name", String::from_utf8_lossy(n));
    }
}
