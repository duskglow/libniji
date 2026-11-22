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

**libniji:** the seven objects point at fixed `"C"` tables (`xtask/src/compat_objects.rs`): identical to the
reference in the `"C"` locale and, for the 16-bit tables, in every UTF-8 locale; they do not follow `setlocale`
(a difference for an 8-bit locale's 16-bit tables and for a UTF-8 locale's `__ctype32_*` entries 128-255; ruling
requested in the LG6-CTYPE-OBJECTS hand-back). Proof: `conformance/c/s05_ctype_legacy_objects.c` (both libraries).
