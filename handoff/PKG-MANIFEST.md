# PKG-MANIFEST: package contents as data (PKG-MANIFEST-3, ready to land once gate2 is green)

State: branch `task/pkg-manifest-3` (continues `task/pkg-manifest-2` and `task/pkg-manifest`), cut
from origin/next 59c31ec19. Role keke `~/w/n2`. Spec: `~/state/briefs/PKG-MANIFEST.md` ("## The
change", "## Proof") plus `~/state/briefs/PKG-MANIFEST-2.md` (packages byte-identical to today).

## Done

* Run 1 (c1d051f0c): `tools/deb/manifest.tsv`, `control.tsv`, `description/<pkg>`,
  `system|coexist/libc-so.ld`, `manifest.py` (lint, rows, packages, debug-package,
  symbols-package, control, member, check).
* Run 2 (5bb3eb4bb): build.sh / system.sh install, strip and write the controls from the tables;
  check-system.sh / check.sh run `manifest.py check`; root-system.sh takes its package count and
  conffiles from the manifest; test `crates/core/tests/deb_manifest.rs`.
* Run 3 (30b04a17c, 32700c07a; handoff commits after):
  - **The red proof was the proof's fault, not a regression.** `pkg-manifest-2-proof` stopped at
    its sanity check `grep -a /usr/lib/niji ld-linux-x86-64.so.2` on the coexist loader. The
    loader does use the prefix: the compiler stores the short `option_env!("NIJI_PREFIX")`
    constant as immediate stores (`strings` shows `lib/nijiH`, split across instructions), so
    the bytes are never contiguous. strace shows the coexist loader opening
    `/usr/lib/niji/etc/ld.so.cache` and `/usr/lib/niji/usr/lib/x86_64-linux-gnu/libc.so.6`. This
    branch changes no product source. The new proof (`~/work/PKG-MANIFEST-3/proof.sh`, copy in
    the session scratch) checks the paths each loader opens under strace instead, and reuses
    PKG-MANIFEST-2's products (built from the 5bb3eb4bb tree, release, full profile).
  - **The byte comparison then found a real defect:** the manifest had no row for `rcrt1.o`
    (product `niji-rcrt1.o`), so libc6-dev and niji6-dev were one file short. The head's own checks
    passed because they read the same manifest. Fixed: row added. Also,
    `manifest.py lint --products` now refuses a product that no file row ships unless
    `UNSHIPPED` names it with why (libc.so: the packages ship libc-so.ld in its place;
    libm.so.6.source-digest: a cache key). deb_manifest's lint test now catches a dropped product.
  - 32700c07a: the reverse check skips symbolic links (the products hold the build's own dev
    links libresolv.so, libmvec.so, libBrokenLocale.so; the packages make links from link rows).
  - Proof `pkg-manifest-3-proof3` on 32700c07a: all 9 .debs IDENTICAL (7 system, 2 coexist),
    check-system / check / check-symbols / root-system PASS, parity inventory IDENTICAL, rc 0.
  - Plants `pkg-manifest-3-plants4`: 8 of 8 CAUGHT, no WARN (results block in
    `handoff/PKG-MANIFEST-plants.txt`). The three plants judged by `manifest.py check` name no test
    (the checker is the test there).

## Jobs (keke)

Final head code 32700c07a: `pkg-manifest-3-proof3` (rc 0), `pkg-manifest-3-plants4` (rc 0),
`pkg-manifest-3-gate2` (clippy, static, `cargo xtask affected --run`). The gate escalates to FULL
by its own rule (`*/libc-so.ld`: "linker or export list, or a script"); the inherited
`pkg-manifest-2-gate` escalated the same way and passed (12079 passed, 0 failed, 138 conformance).
Step 0: `pkg-manifest-3-lint0` (inherited head 856e876: clippy + static, rc 0).
Superseded: `pkg-manifest-3-proof` (found the rcrt1.o defect), `-proof2` (head 30b04a17c), `-gate`
(stopped: 30b04a17c failed the deb_manifest lint on the dev links), `-plants` (refused: a `lit:`
pattern held ` | `), `-plants2` (baseline red: dev links), `-plants3` (two `msg:` regexes did not
match the right failures: WARN-NO-MSG).

## Not to redo / notes

* The libc6-dbg file set is unchanged: iconv is strip `unneeded`.
* Merge demo (run 2): two manifest rows added on head merge clean; the same in the old system.sh
  `for so in` line conflict.
* A proof's "the product names X" check must be behavioural (what the binary opens or prints),
  not a byte grep: constants may be emitted as immediates.
* Process slip, run 3: one heredoc (a python one-off that rewrote the report json; no other effect).
* The plants' system-flavour commands read keke `~/work/PKG-MANIFEST-3/proof/{d-system-base,p-system}`.

## Next step

Land when `pkg-manifest-3-gate2` is green (if red: run only the failing suite). Optional follow-up: a CI step that runs the base-vs-head byte comparison for any change to
tools/deb (the proof script is the recipe).
