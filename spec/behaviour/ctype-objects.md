# ctype-objects

**Spec:** §5.1 (⚑ COMPAT: the table pointers behind the classification macros), §3 (`_obstack`).
**Clauses:** `s0_6_c0_6_0` (the compat-only names old binaries were linked against). Plan row LG-6 `LEGACY-OBJECTS`
(plan/06-phases/18-phase-15-legacy-tier-and.md).

**Question:** what do glibc 2.39's `__ctype_b`, `__ctype_tolower`, `__ctype_toupper`, `__ctype32_b`,
`__ctype32_tolower`, `__ctype32_toupper` and `_obstack` (all compat-only, `GLIBC_2.2.5`) hold for a binary linked
against them, and do they change with `setlocale`? A binary from before the `__ctype_*_loc` accessors reads the
tables through these pointers (a copy relocation of 8 bytes each).

**Probe:** a program binding each object with `.symver` at `GLIBC_2.2.5`, printing every entry -128..255 of the
16-bit class and case tables and 0..255 of the 32-bit ones, comparing each pointer with `*__ctype_*_loc()`, then
the same after `setlocale(LC_ALL, "C.UTF-8")` and `setlocale(LC_ALL, "en_US.UTF-8")`. `readelf --dyn-syms` of the
reference library and of the probe. **Record:** `ctype-objects.glibc.txt` (the `"C"` rows; Zen 1 host, glibc 2.39,
2026-11-22).

**Findings:**

* All seven: `OBJECT GLOBAL`, size 8, `GLIBC_2.2.5` only (no default version); a non-PIE program gets a
  `R_X86_64_COPY` of each.
* In the `"C"` locale each ctype pointer equals the matching `*__ctype_*_loc()` (`__ctype32_tolower` and
  `__ctype32_toupper` equal `__ctype_tolower` and `__ctype_toupper`); the class table is indexable from -128, with
  no class outside 0-127; a case table maps -128..-2 to c + 256, -1 to -1, and every byte outside A-Z/a-z to itself.
  `__ctype32_b[c]` is the 16-bit mask's classes in the `_ISwbit` encoding. `_obstack` is NULL.
* **After `setlocale`** the program's copies follow the new global locale: each 16-bit pointer still equals
  `*__ctype_*_loc()`, and the `__ctype32_*` pointers move to the locale's own tables. For `C.UTF-8` and
  `en_US.UTF-8` the 16-bit tables' contents equal the `"C"` ones; the `__ctype32_*` tables differ at 128-255
  (the code points U+0080..U+00FF). An 8-bit locale would change the 16-bit tables too (none installed on the probe
  host).

**setlocale probe (CTYPE-OBJECTS-SETLOCALE, Zen 1 host, glibc 2.39, 2026-11-23):** the same binding, after each step
below; LOCPATH holds `de_DE.ISO-8859-1`, `en_US.UTF-8`, `ru_RU.KOI8-R` compiled with the host's `localedef` and a copy of
`C.utf8`. "16 = loc": the three 16-bit pointers equal `*__ctype_*_loc()`; "32 = items": the three 32-bit pointers equal
`nl_langinfo` items 5, 16, 15 (`_NL_CTYPE_CLASS32`, `_TOLOWER32`, `_TOUPPER32`).

| step | 16-bit objects | 32-bit objects |
|---|---|---|
| start ("C") | = loc, "C" tables | = items; `__ctype32_tolower`/`toupper` = the 16-bit pointers |
| `setlocale(LC_ALL, "en_US.UTF-8")` or `"C.UTF-8"` | = loc (contents as "C") | = items: U+0000..U+00FF by code point (`toupper32[0xb5]` = 0x39c, `[0xff]` = 0x178) |
| `setlocale(LC_ALL, de ISO-8859-1)`, `LC_CTYPE` alone, or `""` with `LC_ALL` = it | = loc (0xe9 a lower-case letter) | = items, the same contents as en_US.UTF-8's |
| `setlocale(LC_ALL, "ru_RU.KOI8-R")` | = loc | = items, the same contents as en_US.UTF-8's (by code point, not byte) |
| `setlocale(LC_NUMERIC, de)` alone | unchanged ("C") | unchanged |
| `uselocale(de object)`, main thread or another | unchanged (the global locale's), != loc | unchanged |
| back to `"C"` or `"POSIX"` | the start pointers | the start pointers |

**libniji (CTYPE-OBJECTS-SETLOCALE):** `setlocale`'s cold path (only when the global locale changed) stores the global
`LC_CTYPE`'s tables into the six objects through the GOT of their versioned names (an old binary's COPY is written):
the 16-bit ones `*__ctype_*_loc()`'s, the 32-bit ones a locale file's items, else the "C" tables. Left (D-5.13,
docs/differences-from-glibc.md): before the first change of the global `LC_CTYPE` the objects point at libniji's own
copies of the "C" tables (same contents, other addresses), and the built-in `"C.UTF-8"` gives the "C" 32-bit tables.
Proof: `conformance/c/s05_ctype_legacy_objects.c` (both libraries; every row above except the UTF-8 and KOI8-R ones, which
share the locale-file path of the de row).
