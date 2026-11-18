# LAYOUT-LDSO-ORDER: ld.so gets a traced, ordered start-up layout like libc

State: branch `task/layout-ldso-order-3` (continues `task/layout-ldso-order-2` and `task/layout-ldso-order`), run 3.
Base origin/next 627ef49ca. Role mei:~/w/s3 (runs 1-2: keke:~/w/n1).
Brief: ~/state/briefs/LAYOUT-LDSO-ORDER-2.md ("## The change"), narrowed by ~/state/briefs/LAYOUT-LDSO-ORDER-3.md.
Decision: records/pending/LAYOUT-LDSO-ORDER-2-1.toml (D-NEW, series D-X; CI assigns the id).
Rulings in force (coordinator 11-18): #6504 the N=0 +347 instructions (0.004%) are an accepted measured cost against
-3 faults; #6505 the text ordering lands; the insensitivity property, ld.so data and the 64 KiB window phase move to
the new story LAYOUT-LDSO-WINDOW. **This story does not claim insensitivity to unrelated ld.so growth.**

## Done
* Run 1: tracer mode `HOT_TEXT_LDSO`, `tools/hottext/ldso_trace.sh` (`true`, `libs3`; slow and numa1 traces must
  equal), `group_ldso`/`ldso_lists` in xtask/src/hottext.rs, traced link in xtask/src/so.rs (details in the D-NEW body).
* Run 2 (counts on keke, Zen 1, fast products, 21 runs, paths of one length; logs ~/logs/layout-ldso-order-2-*.log):

  | N libs | faults ASLR off base -> new | ASLR on median base -> new | instructions:u median base -> new |
  |---|---|---|---|
  | 0 | 81 -> 78 | 82 -> 79 | 9037737 -> 9038084 (+347) |
  | 3 | 97 -> 95 | 97 -> 95 | 9317884 -> 9317882 |
  | 12 | 133 -> 131 | 134 -> 131 | 10120229 -> 10120228 |

  Page split (N=0): ld.so text faults 5 -> 2; data and anonymous pages unchanged. Determinism: Zen 1 and Skylake-SP
  builds give byte-identical lists, signatures, bins, ld.so and libc.so.6. A 46 KiB `#[cold]` pad still adds +4
  faults on base and branch (window phase, +1 RW page, +2 anonymous): handed to LAYOUT-LDSO-WINDOW.
* New test `init_dynamic_startup_only_program_stays_within_its_page_fault_bound` (hot_text_products).
* Run 3: lint on the inherited head green (`layout-ldso-order-3-lint0`: clippy, static, records check). The +347 cause
  search (bounded, per ruling): the base and new interpreters have identical section addresses except `.text`
  (0x46b80 -> 0x46a9e, 226 bytes shorter grouped); libc.so.6 identical. So only code placement differs; the cause was
  not found, and the record states it as the measured cost. The plants file no longer runs `cargo xtask so` inside the
  plant commands (the tool rebuilds traced for a layout test), and the xtask plant names `--bin xtask`.
* Run 3 jobs on mei (head b4bb760eb): `layout-ldso-order-3-plants` (handoff/LAYOUT-LDSO-ORDER-2-plants.txt,
  /mnt/zram/claude/layout-ldso-order-3/plants-1, --timeout 3600) and `layout-ldso-order-3-gate` (direct tier
  `--base origin/next`, then `cargo xtask test`, then `NIJI_FULL_PROFILE=1` so + hot_text_products). Results: see
  handoff/reports/LAYOUT-LDSO-ORDER-3.json.

## Next step
Land (train). Follow-up: LAYOUT-LDSO-WINDOW (window phase independent of image size, e.g. keep the interpreter's
mapped size a multiple of 64 KiB or align its base; ld.so data/bss pages; the insensitivity proof with the pad).

Do not redo: base/new/pad builds and counts (run 2), the +347 search.
