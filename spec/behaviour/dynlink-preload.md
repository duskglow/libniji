# dynlink-preload

**Spec:** §27 (the dynamic loader), ld.so(8) "LD_PRELOAD" and "/etc/ld.so.preload",
`plan/ph14/threat-model-draft.md` rule 1 (the secure-mode exception) and rule 5 (trusted
directories). **Story:** DL-LD-PRELOAD. Secure-mode facts are in `dynlink-secure.md` (DL-0a); this
record covers the rest.

**Question:** in what order the reference loader puts preloaded objects in the scope and runs their
constructors, how `LD_PRELOAD` and the preload file are split into names, what it prints for an object
it cannot preload, and what a start costs when there is no preload. A fact record, not a decision.

**Probe:** `tools/probe/dl/preload/run.sh SCRATCH` (the corpus, `LD_PRELOAD` cases, the preload file in
a user + mount namespace that binds a private copy of `/etc`), then `run2.sh SCRATCH` (the file's
comment and separator details). Run on a Zen 1 host against the reference (GLIBC 2.39) as jobs
`dl-ld-preload-probe1` and `dl-ld-preload-probe2`. Each test library prints `ctor <name>` from a
constructor and defines `f` (`dep` 2, `pre` 1, `pre2` 5, `epre` 3); `libpre` also defines `strlen` as
`dlsym(RTLD_NEXT, "strlen")` plus 1000. The program needs `libdep` (found through its RPATH) and prints
`f=<f()> strlen=<strlen(argv[0])>`.

## Order and scope

- Scope: the program, the `LD_PRELOAD` names in order, the preload file's names in order, then the
  program's `DT_NEEDED` objects (breadth first). The first preload's `f` wins over `libdep`'s.
- `dlsym(RTLD_NEXT, "strlen")` from the preload reaches libc's `strlen` (the program prints its length
  plus 1000).
- Constructors: a preload gets no dependency edge from the program; the order is the usual one over
  the objects in load order. `LD_PRELOAD="<d>/libpre2.so:  <d>/libpre.so"`: `ctor dep`, `ctor pre`,
  `ctor pre2`, `ctor prog`. File `libepre.so` with `LD_PRELOAD=libpre2.so`: `ctor dep`, `ctor epre`,
  `ctor pre2`, `ctor prog`.
- The same object twice, or an object that is also a dependency of the program: loaded once.

## Names

- `LD_PRELOAD`: separated by spaces and colons only (a tab or newline is part of a name); empty names
  skipped (`' : '` preloads nothing). `$ORIGIN` expands (the program's directory). A name without a
  slash is searched like a `DT_NEEDED` name of the program (its RPATH found `libpre.so`).
- The preload file `/etc/ld.so.preload`: separated by spaces, tabs, newlines and colons (comma and
  carriage return are part of a name); `#` starts a comment to the end of its line, also when glued to
  a name (`a.so#b.so` preloads `a.so` only); no final newline needed; an empty file preloads nothing.
  Its names come after the `LD_PRELOAD` names.

## An object that cannot be preloaded

The program runs; standard error gets one line per object:
`ERROR: ld.so: object '<name>' from <source> cannot be preloaded (<reason>): ignored.` where `<source>`
is `LD_PRELOAD` or `/etc/ld.so.preload`, and `<reason>`:

| object | reason |
| --- | --- |
| missing | `cannot open shared object file` |
| a short file that is not ELF | `file too short` |
| a directory | `cannot read file data` |
| an ELF file cut to 100 bytes | `cannot read file data` |

## Cost

Every dynamically linked start checks for the preload file once (`access("/etc/ld.so.preload", R_OK)`:
one system call, `ENOENT` when absent). libniji opens it instead (one `openat`, `ENOENT` when absent):
the same count.

## Clauses covered

A fact record: it fixes no clause text. It bears on s27_1_g_0 (symbol resolution in global scope
order: the preloads come before the program's dependencies), s27_1_c_3 (`RTLD_NEXT` from a preload)
and s1_1_g_4 (secure mode: `LD_PRELOAD` is the one name read before the clear, under the threat
model's rule 1).

## libniji

`crates/ld/src/start.rs` (`preload_names`, `preload_file_names`, `preload_rule`, `preload_mode_admits`,
`preload_ignored`) and `crates/boundary/src/ld.rs` (`Preloads`, `link`); tests
`crates/ld/tests/preload.rs` and `crates/ld/tests/ldso_run.rs`.
