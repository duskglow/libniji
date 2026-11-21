# VALGRIND-STRING-REDIRECT: memcheck replaces libniji's string functions (GitHub #13)

Branch `task/valgrind-string-redirect` (from origin/next 83f91cb05). Brief `~/state/briefs/VALGRIND-STRING-REDIRECT.md`.
Role mei `~/w/s1`. Job scripts and outputs: mei `~/work/VALGRIND-STRING-REDIRECT/` (acc.sh, chain.sh, vs/{run,pre,post}.sh,
vs/strs.c, vs/over.c; acc-head/, acc-before/; vgnames.py: valgrind's view of a libc.so.6 `.symtab`). Decision D-X.154.
Plants `handoff/VALGRIND-STRING-REDIRECT-plants.txt`. Report `handoff/reports/VALGRIND-STRING-REDIRECT.json`.

## Cause (valgrind 3.24 source, read as the brief allows; no glibc source)

Memcheck binds each replacement to the start address of a function symbol carrying a name it lists for a `libc.so*`
soname (`shared/vg_replace_strmem.c`). Its reader drops function symbols of size 0 (`m_debuginfo/readelf.c`);
`storage.c`'s `canonicaliseSymtab` merges symbols with the same start and size (all names kept) and, of two sharing a
start, moves the longer past the shorter. Two gaps in libc.so.6:

1. `strcmp`, `strncmp`, `strchr`, `index`, `strchrnul`: size-0 labels on their kernels (in .dynsym and .symtab), so
   memcheck saw only `niji_machine_*` there: PLT calls ran the vector kernels (ls: niji_machine_strcmp).
2. libc's own calls reach the `strlen`, `strnlen`, `memchr`, `rawmemchr` kernels directly (strz.rs), not through the
   exported entry (`strlen` is an 18-byte Rust entry; `memchr`/`rawmemchr` have a gate before the kernel): never
   replaced (find: niji_machine_strlen).

Every other exported replaced name was already visible (vgnames.py over the head product: 0 MISS after the fix).

## The change (code commit d0b2393aa; D-X.154 in f9dd3abc1)

* `csys/src/machine/scan.S`: `.size` for the five labels (and `niji_machine_strcmp_entry`); under NIJI_EXPORT, local
  sized aliases `__GI_strlen`, `__GI_strnlen`, `__GI_memchr`, `__GI___rawmemchr` on the four kernels (names from
  valgrind's libc list that no export uses; a local copy of the exported name would make by-name `.symtab` lookups
  ambiguous). Symbol table only: no code byte; the kernels' addresses are the same as before the change
  (0x31dc0, 0x51091, 0x544c0 in the fast products).
* `xtask/src/so.rs`: `check_valgrind_string_names` after the libc.so.6 link: emulates valgrind's view of `.symtab` and
  refuses an exported replaced name not visible at its address, or a kernel without its alias. Unit test
  `valgrind_string_names_resolve_to_the_bodies`.
* Not aliased (on purpose): kernels whose result is not a replaced function's (niji_machine_memcmp returns an index,
  strdiff, casecmp_ord_*, pairscan*, strpair, wcspair): a replacement there would compute the wrong thing.
  memrchr's inner kernel: only the memrchr entry's rest path calls it (redirected under valgrind), so no alias.

## Acceptance (job valgrind-string-redirect-acc; swapped trixie root, valgrind 3.24.0, whole libc6-dbg)

| run (memcheck errors) | ls / | ls -l /etc | find /etc -name passwd | sort /etc/passwd | strs (every replaced fn) | over (real overread) |
|---|---|---|---|---|---|---|
| stock (trixie libc6 2.41) | 0 | 0 | 0 | 0 | 0 | 1 (in strlen) |
| before (2.39+niji20261121.1 debs) | 12 | 14 | 72 | 0 | 12 (niji_machine_strncmp) | 1 |
| head | 0 | 0 | 0 | 0 | 0 | 1 (in strlen) |

(The issue's 21.6 counts were 5, 7 and 4; the before column is the 21.1 snapshot's debs, the same mechanism.)
`--trace-redir=yes` on strs, head: active redirections in libc.so.6 for every exported replaced name above, plus
`__GI_strlen`, `__GI_strnlen`, `__GI_memchr`, `__GI___rawmemchr`. check-system PASS (7 packages).

## Jobs (mei, code head f9dd3abc1; later commits handoff/ only)

* `valgrind-string-redirect-gate`: direct tier (FULL: the train's, xtask/src/so.rs), 16 suites ok, 11263 passed,
  0 failed; conformance 142/0; peak 15G.
* `valgrind-string-redirect-plants2`: 3 plants, 3 CAUGHT. (`-plants` superseded: started before the sync, no file.)
* `valgrind-string-redirect-acc`: debs from the head (check-system PASS) + acceptance above, before and after.
* `valgrind-string-redirect-lint`: clippy (workspace, all targets/features, -D warnings) + `cargo xtask static`.

## Open

* `__stpcpy_chk`: the spec is loaded but no binding is active (stock binds it); also so before this change.
  `__stpcpy_chk` and `niji_machine_stpcpy_chk` are visible at 0xc4740 (size 597) in the emulation. Next: read
  m_redir.c's binding conflict rules against `--trace-redir=yes -v` output (acc-head/out/niji-redir.err).
* Per-call counts not re-measured: no code byte changed (symbol-table only); not verified with a bench job.
* helgrind (VALGRIND-DBG-RODATA item 2) and the leak-frame view mapping: untouched here.
