# LDCONFIG-DEV-ALIAS: ldconfig never points a soname link back into its own chain (GitHub issue #16)

**State.** Branch `task/ldconfig-dev-alias`, cut from origin/next e40107432. Code head 0d79a065f (two commits: the fix,
then its generalisation found by step 4); later commits only add this note and the report json. Role mei `~/w/s1`.

## What the probes settled (black box, the reference ldconfig in private roots, transcript harness)

The brief guessed the defect was the candidate rule (an alias that is not a prefix of the soname becomes a candidate).
The probes say otherwise: the reference treats a resolving `.so` alias as a candidate too. What it does differently is
**leave a soname path alone when it already resolves to the winner's file** (same device and inode), silently and
whatever its text. libniji compared the link's text, so it rewrote `libcmocka.so.0` to `libcmockery.so`, whose chain
runs back through `libcmocka.so.0`: a loop.

| case (dir in `r-dev-alias-cache.transcript`) | reference | next | this branch |
|---|---|---|---|
| issue #16, soname link present (a; also `n-dev-alias`) | `-v` `libcmocka.so.0 -> libcmockery.so`, link kept | rewrote the link to `libcmockery.so` (changed): loop | = reference |
| issue #16, soname link absent (`n-dev-alias-no-link`) | aliases dangle, link -> `libcmocka.so.0.8.0` | = | = |
| `libfoo-compat.so` alias, sorts below the file, link present (b) | link kept | = | = |
| `libquxalt.so` alias, sorts above the file, no link (c) | link made -> `libquxalt.so` (changed) | = | = |
| `.so` regular file, versioned soname (d) | `libbar.so.3 -> libbar.so` (changed) | = | = |
| `.so` regular file, own soname (e) | cached under its name, no link | = | = |
| upgrade: link at 1.0, 1.1 present, alias through the links (f) | `-v` names the alias, link kept at 1.0 | rewrote to the alias | = reference |
| link `./libsf.so.1.2` (other text, same file) (g) | kept, not changed | rewritten (changed) | = reference |
| alias above the file, link present (h) | `-v` names the alias, link kept | rewrote to the alias | = reference |
| link at old version, no alias (i) | moved to 1.1 (changed) | = | = |
| regular file at the soname path = winner's file (j; trixie's `libtcl8.6.so`) | silent | "is not a symbolic link" | = reference |
| cache entries (all dirs) | prefix dev link has its own entry; aliases none | = | = |

Every row is equal on this branch (23/23 ldconfig transcripts ok against the reference), so no differences row.
The rule is written in `crates/ld/src/ldconfig/mod.rs` as libniji's rule. Why no loop can form: a winner whose chain
passes through the soname link resolves to whatever that link resolves to, so the identity check keeps the link.

## Proof

* Issue reproducer: `n-dev-alias` (present) and `n-dev-alias-no-link` (absent) transcripts: `libcmocka.so.0 ->
  libcmocka.so.0.8.0` both orders. On next: `n-dev-alias` and `r-dev-alias-cache` MISMATCH (job
  ldconfig-dev-alias-check-next: a, f, g, h rewritten), `n-dev-alias-no-link` ok.
* Engine test `a_soname_link_reaching_the_winner_is_kept` (crates/ld/tests/ldconfig.rs): issue both orders, f, g, h,
  i, j. Plants `handoff/LDCONFIG-DEV-ALIAS-plants.txt` (fix reverted, dev-only identity, stat of the wrong path,
  identity only for links), job ldconfig-dev-alias-plants.
* Step 4: `ldconfig -n -v` over a RAM copy of the trixie host's `/usr/lib/x86_64-linux-gnu` (725 entries, 343
  symlinks), libniji's engine and the reference on separate copies: 0 "changed" lines on both, `-v` output identical
  (381 lines, sorted), 0 links differ from each other or from the original, stderr identical after the program-name
  normalisation (job ldconfig-dev-alias-check3; the first run, trixie1, found the `libtcl8.6.so` message, fixed in
  0d79a065f).
* check3 on 0d79a065f: workspace clippy -D warnings, xtask clippy, static, 10 engine tests, 23/23 transcripts.
* Gate ldconfig-dev-alias-gate (direct tier) on 0d79a065f: 3 suites, 97 passed, 0 failed, peak 1.6G. Plants ldconfig-dev-alias-plants: 4/4 CAUGHT, no WARN.

## Open questions

* RULING NEEDED (owner/coordinator): issue #16's "Expected" says `.so` names are never soname-link targets. The
  reference does make them targets when the existing link does not already reach the winner (c: a new link to
  `libquxalt.so`, which dangles if the -dev package goes; f: an alias reached through the old link keeps the soname
  link at the old version). This branch follows the reference (the reported loop is gone in every order).
  Recommendation: keep it so; the issue's failure is fixed and its real layouts (Debian dev links) behave as the
  reference does. Excluding `.so` symlinks from candidates would differ in c and f and need a differences row and a
  compat flag.
* Not probed: `-l` operands (`Run::libs`) still compare link text; no report says that matters.

## Do not redo

The probe transcripts are recorded against the reference on the trixie host; re-record only with a reason.
The trixie check script was `~/work/LDCONFIG-DEV-ALIAS/trixie.sh <n>` on mei (not in the tree).

## Next step

Land. Reply on issue #16: fixed; the rule is "a soname link that already reaches the winner's file is kept".
