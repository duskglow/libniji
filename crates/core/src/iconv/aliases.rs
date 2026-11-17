//! Charset names that name an installed charmap without being one of its own names (spec §6.4,
//! `spec/encoding/iconv-names.md`, rule 5).
//!
//! `iconv_open` matches a name exactly ([`super::names_charset`]), so a spelling the system knows
//! only in another form (`KOI8R` for the charmap `KOI8-R`) needs a name of its own. Each row is
//! a name `iconv -l` lists on the reference system and the charmap (file name, matching form)
//! that holds the same conversion: the two names reduce to the same letters and digits, and a
//! black-box probe of the reference decodes every one- and two-byte input and encodes every
//! code point (all of the BMP, a stride of the rest) the same way through both. The table does
//! not depend on which charmap files are installed: a name whose charmap is missing names
//! nothing, as the charmap's own name does. Where a generated charmap already carries a row's
//! name, the row points to that charmap: `HPTHAI8` is `ISO-8859-11`, which the probe found
//! identical to `HP-THAI8`.
//!
//! Sorted by name for a binary search; the core test `s6_4_c_1_charmap_aliases` checks the
//! order, that no row is a built-in name, and that the generated charmaps
//! (`spec/encoding/charmaps.tsv`) never give a row's name to another charmap.

/// (name in matching form, charmap file name in matching form), sorted by name.
const CHARMAP_ALIASES: &[(&[u8], &[u8])] = &[
    (b"ARMSCII8", b"ARMSCII-8"),
    (b"BIG-5", b"BIG5"),
    (b"CSA_Z243.419851", b"CSA_Z243.4-1985-1"),
    (b"CSA_Z243.419852", b"CSA_Z243.4-1985-2"),
    (b"DECMCS", b"DEC-MCS"),
    (b"EBCDICATDE", b"EBCDIC-AT-DE"),
    (b"EBCDICATDEA", b"EBCDIC-AT-DE-A"),
    (b"EBCDICCAFR", b"EBCDIC-CA-FR"),
    (b"EBCDICDKNO", b"EBCDIC-DK-NO"),
    (b"EBCDICDKNOA", b"EBCDIC-DK-NO-A"),
    (b"EBCDICES", b"EBCDIC-ES"),
    (b"EBCDICESA", b"EBCDIC-ES-A"),
    (b"EBCDICESS", b"EBCDIC-ES-S"),
    (b"EBCDICFISE", b"EBCDIC-FI-SE"),
    (b"EBCDICFISEA", b"EBCDIC-FI-SE-A"),
    (b"EBCDICFR", b"EBCDIC-FR"),
    (b"EBCDICISFRISS", b"EBCDIC-IS-FRISS"),
    (b"EBCDICIT", b"EBCDIC-IT"),
    (b"EBCDICPT", b"EBCDIC-PT"),
    (b"EBCDICUK", b"EBCDIC-UK"),
    (b"EBCDICUS", b"EBCDIC-US"),
    (b"ECMACYRILLIC", b"ECMA-CYRILLIC"),
    (b"EUCJP-MS", b"EUC-JP-MS"),
    (b"EUCKR", b"EUC-KR"),
    (b"EUCTW", b"EUC-TW"),
    (b"GB_198880", b"GB_1988-80"),
    (b"GOST_1976874", b"GOST_19768-74"),
    (b"GREEK7OLD", b"GREEK7-OLD"),
    (b"GREEKCCITT", b"GREEK-CCITT"),
    (b"HPGREEK8", b"HP-GREEK8"),
    (b"HPROMAN8", b"HP-ROMAN8"),
    (b"HPROMAN9", b"HP-ROMAN9"),
    (b"HPTHAI8", b"ISO-8859-11"),
    (b"HPTURKISH8", b"HP-TURKISH8"),
    (b"IBM-1124", b"IBM1124"),
    (b"IBM-1129", b"IBM1129"),
    (b"IBM-1132", b"IBM1132"),
    (b"IBM-1133", b"IBM1133"),
    (b"IBM-1160", b"IBM1160"),
    (b"IBM-1161", b"IBM1161"),
    (b"IBM-1162", b"IBM1162"),
    (b"IBM-1163", b"IBM1163"),
    (b"IBM-1164", b"IBM1164"),
    (b"IBM-856", b"IBM856"),
    (b"IBM-922", b"IBM922"),
    (b"IEC_P271", b"IEC_P27-1"),
    (b"INIS8", b"INIS-8"),
    (b"INISCYRILLIC", b"INIS-CYRILLIC"),
    (b"ISIRI3342", b"ISIRI-3342"),
    (b"ISO11548-1", b"ISO_11548-1"),
    (b"ISO8859-10", b"ISO-8859-10"),
    (b"ISO8859-11", b"ISO-8859-11"),
    (b"ISO8859-13", b"ISO-8859-13"),
    (b"ISO8859-14", b"ISO-8859-14"),
    (b"ISO8859-15", b"ISO-8859-15"),
    (b"ISO8859-16", b"ISO-8859-16"),
    (b"ISO8859-2", b"ISO-8859-2"),
    (b"ISO8859-3", b"ISO-8859-3"),
    (b"ISO8859-4", b"ISO-8859-4"),
    (b"ISO8859-5", b"ISO-8859-5"),
    (b"ISO8859-6", b"ISO-8859-6"),
    (b"ISO8859-7", b"ISO-8859-7"),
    (b"ISO8859-8", b"ISO-8859-8"),
    (b"ISO8859-9", b"ISO-8859-9"),
    (b"ISO8859-9E", b"ISO-8859-9E"),
    (b"ISO885910", b"ISO-8859-10"),
    (b"ISO885911", b"ISO-8859-11"),
    (b"ISO885913", b"ISO-8859-13"),
    (b"ISO885914", b"ISO-8859-14"),
    (b"ISO885915", b"ISO-8859-15"),
    (b"ISO885916", b"ISO-8859-16"),
    (b"ISO88592", b"ISO-8859-2"),
    (b"ISO88593", b"ISO-8859-3"),
    (b"ISO88594", b"ISO-8859-4"),
    (b"ISO88595", b"ISO-8859-5"),
    (b"ISO88596", b"ISO-8859-6"),
    (b"ISO88597", b"ISO-8859-7"),
    (b"ISO88598", b"ISO-8859-8"),
    (b"ISO88599", b"ISO-8859-9"),
    (b"ISO88599E", b"ISO-8859-9E"),
    (b"ISO_10367BOX", b"ISO_10367-BOX"),
    (b"ISO_5427EXT", b"ISO_5427-EXT"),
    (b"ISO_8859-14", b"ISO-8859-14"),
    (b"ISO_8859-9E", b"ISO-8859-9E"),
    (b"JIS_C62201969RO", b"JIS_C6220-1969-RO"),
    (b"JIS_C62291984B", b"JIS_C6229-1984-B"),
    (b"KOI8", b"KOI-8"),
    (b"KOI8R", b"KOI8-R"),
    (b"KOI8U", b"KOI8-U"),
    (b"LATIN9", b"ISO-8859-15"),
    (b"LATINGREEK", b"LATIN-GREEK"),
    (b"LATINGREEK1", b"LATIN-GREEK-1"),
    (b"MACCYRILLIC", b"MAC-CYRILLIC"),
    (b"MACIS", b"MAC-IS"),
    (b"MACUK", b"MAC-UK"),
    (b"NATSDANO", b"NATS-DANO"),
    (b"NATSSEFI", b"NATS-SEFI"),
    (b"NC_NC0010", b"NC_NC00-10"),
    (b"NF_Z_62010", b"NF_Z_62-010"),
    (b"NF_Z_62010_1973", b"NF_Z_62-010_1973"),
    (b"NS_45511", b"NS_4551-1"),
    (b"NS_45512", b"NS_4551-2"),
    (b"SHIFT-JIS", b"SHIFT_JIS"),
    (b"SHIFTJISX0213", b"SHIFT_JISX0213"),
    (b"T.618BIT", b"T.61-8BIT"),
    (b"WINSAMI2", b"SAMI-WS2"),
];

/// The charmap `name` (matching form, [`super::Spec::name`]) stands for when it is not a name of
/// that charmap itself, if any.
pub fn charmap_alias(name: &[u8]) -> Option<&'static [u8]> {
    let i = CHARMAP_ALIASES.binary_search_by(|(a, _)| (*a).cmp(name)).ok()?;
    CHARMAP_ALIASES.get(i).map(|(_, target)| *target)
}

/// Every row, in order: (name, charmap).
pub fn charmap_aliases() -> impl Iterator<Item = (&'static [u8], &'static [u8])> {
    CHARMAP_ALIASES.iter().copied()
}
