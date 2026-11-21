# CUTOVER-RESIDUE-AUDIT-B: what 28 stories that died around the 10-24 cutover never landed

Audit only; no code changed. Base: origin/next 43058b7df (2026-11-21). Verdict: **nothing is missing. Close all 28.**
No new story and no continuation is needed. Two of the 28 (MATH-M5-CPLX-F128, MATH-M5-CPLX-F128-ZIV-A-3) were
already closed by the 11-20 red triage a few minutes after this brief was written; they are confirmed here.

## Method

1. **The public root contains historic next in full.** The public root commit `7374bf3f2` (the public tree,
   10-24 19:41) differs from `historic/next` `76041c218` (10-24 19:31) only by 784 deleted files, all under
   `handoff/` (471 of them `handoff/reports/`). `git diff --name-only historic/next 7374bf3f2 | grep -v ^handoff/`
   is empty. So a branch with 0 commits ahead of `historic/next` reached the public tree in full, apart from its
   handoff notes. This settles the "NOT VERIFIED: lands through INTEGRATE-* really in the public root" caveat of
   `handoff/cutover-reds-triage.md` (11-04) for every such branch.
2. **Ancestry into the chain.** Each story's branch exists only on `historic/task/<id>`. For each one: its commits
   ahead of `historic/next`, and whether it is an ancestor of the chain's next link (`git merge-base --is-ancestor`).
   The next links were each checked line by line against next by HELD-REDS-TRIAGE (11-05,
   `handoff/held-reds-triage/summary.md`).
3. **Content against today's next.** Each branch's added lines outside `handoff/` (diff against its merge base with
   `historic/next`) were looked up in two places: the same file on origin/next, then anywhere in the origin/next
   tree (file moves and splits). The project rename (former name -> `niji` in identifiers) was normalised first. Every remaining unmatched group
   was read by hand and its later form found on next (table below).
4. **Symbols.** The compat names the LG stories targeted were checked in `spec/abi/compat-plan.tsv` (declared) and
   `spec/abi/compat-missing.tsv` (absent). Exports were checked with `readelf --dyn-syms` on a dynamic `libc.so.6`
   built 11-21 on the Zen 1 build host. That product came from a story branch cut from next, not from next itself.

## Ranked table

Ranked by how much residue there was to explain (most first). "Own / present" counts the branch's added lines
outside handoff/ (normalised) and how many of them are on origin/next: same file + elsewhere.

| # | Story | Branch vs next | Still missing | Why it would matter | Size | Recommendation |
|---|---|---|---|---|---|---|
| 1 | MATH-M6-FPENV-HONOR-3 | 37 ahead; ancestor of -4 (CONTAINED 11-05); 160/255 lines | nothing. Unmatched: an older `s10_math_symbols.rs` test set, the MXCSR-read sqrt gate in `mathret.rs`/`fpu.rs`, one D-10.22 wording. Next runs `sqrtsd` on the caller's MXCSR without reading it (D-10.101). The libm-exact paragraph was folded into MATH-M6-F32-BENCH-2-R-2 (landed 11-05) | nothing | none | close (superseded by -4, then D-10.101) |
| 2 | MATH-M6-SQRT-FTZ-5 | 18 ahead; ancestor of -6 and FPENV-HONOR-4; 243/302 | nothing. Unmatched: the `is_sqrt_in_place` gate, a separate `machine_sqrtss` entry, old tests. Superseded by D-10.22 (owner ruling 10-23), then D-10.101 (no MXCSR read) | nothing | none | close |
| 3 | MATH-M6-F64-GAP-ROUND-TEST | 18 ahead; ancestor of -2 (CONTAINED); 623/726 | nothing. Unmatched: `elem_fast` wiring in `s10_math.tsv`/`_gen.rs` (replaced by the hot.rs dispatch), the register section's old numbering (now `docs/saturation/16-f64-exp-log-pow-gap-breakdown.md`), rewritten `fast64.rs` lines. `near`, `log_eps`, `exp_eps`, `reduce_exp`, `reduce_pow`, `logb_near` are all in fast64.rs | nothing | none | close |
| 4 | MATH-M6-F64-GAP-EXP-REDUCE | 12 ahead; ancestor of -2 (CONTAINED); 466/546 | nothing. Same pattern as row 3 (Cody-Waite `reduce_exp` on next) | nothing | none | close |
| 5 | MATH-M6-F32-BENCH | 15 ahead; ancestor of -2; 371/449 | nothing. Unmatched: older `fast32.rs` constants and cores. The cores are on next in `fast32/exp_log.rs` (`log1p_core`, `pow_core`); `TRIG_DELTA` etc. are in fast32.rs. `validation/percall/math_f32.c` landed through MATH-M6-F32-BENCH-2-R-2 (11-05) | nothing | none | close |
| 6 | MATH-M6-F64-GAP-ASSEMBLY | 6 ahead; ancestor of -2 (CONTAINED); 373/447 | nothing. Same pattern as row 3 (`log_assemble`, `exp_nearest` on next) | nothing | none | close |
| 7 | TIME-SPEC-GAPS-SETTLE-2 | 6 ahead; ancestor of -3 (CONTAINED); 301/336 | nothing. Unmatched: the old probe harness, and the strptime signed-number row. libniji later changed that row to match the reference (STRPTIME-SIGNED-YEAR, GitHub issue 18): `spec/behaviour/time-gaps.md:28` "Equal" | nothing | none | close |
| 8 | PERF-FNMATCH-PATHNAME-2 | 8 ahead; ancestor of -3 (CONTAINED); 640/674 | nothing. Unmatched: merged cpow work that next has moved past, and a superseded register status line | nothing | none | close |
| 9 | PERF-GETC-PUTC-TRIMS | 4 ahead; ancestor of -2 (CONTAINED); 27/52 | nothing. The `getc_slow` split lives on as `getc_slow`/`getc_slow_general` (`crates/boundary/src/stdio.rs:795-802`). The register section is now `docs/saturation/14-...md` row 9 (N2-9) | nothing | none | close |
| 10 | MATH-M5-CPLX-F128-ZIV-A-3 | 12 ahead; ancestor of ZIV-A-4; 897/916 | nothing. Unmatched: f128 `fdlq` rows (now `lq` rows, `s10_math.tsv:191-200`), `TWO_OVER_PI` (now `t::TWO_OVER_PI`), wording. The chain landed as MATH-M5-CPLX-F128-ZIV-C-4 (11-04) | nothing | none | already closed (11-20) |
| 11 | MATH-M5-CPLX-F128 | 3 ahead; 173/178 | nothing. Same as row 10 (`complex.rs` split into `complex_body.rs`; `cacosf128` etc. in `math_libm.c:109-110`) | nothing | none | already closed (11-20) |
| 12 | HOSTCONF-OPTIONS-PROBE-2 | 5 ahead; ancestor of -3 (CONTAINED); 407/423 | nothing. The on/off parser was rewritten as `on_off`/`on_off_rest` (`hostconf.rs:131-136`). Test addresses moved to documentation ranges (192.0.2.x) | nothing | none | close |
| 13 | HOSTCONF-DIAGNOSTICS | 8 ahead; ancestor of -2 (CONTAINED); 572/581 | nothing. Same test-address and wording changes | nothing | none | close |
| 14 | NS-NETWORKS-DNS | 4 ahead; ancestor of -2 (CONTAINED); 387/388 | nothing. One wording line; COMPAT-13 and its differences row are on next | nothing | none | close |
| 15 | PERF-MEMSET-4K-SSE2 | 4 ahead; ancestor of -2 (CONTAINED); 65/65 | nothing | nothing | none | close |
| 16 | COMPAT-NSS-RESOLV-PT2-8 | 0 ahead (in the public root) | nothing. The 8 libresolv compat names (`_gethtby*`, `_gethtent`, `_sethtent`, `res_gethostby*`) are `libresolv.so.2` rows in compat-plan.tsv, wired in `crates/legacy/src/resolv_compat.rs`, none in compat-missing.tsv. NOT VERIFIED by readelf (the product set checked had no libresolv.so.2) | old binaries linked against `libresolv.so.2`'s 2.2.5 host-table calls | none | close |
| 17 | COMPAT-NSS-RESOLV-3 | 0 ahead | nothing. `__nss_hosts_lookup@GLIBC_2.2.5`, `__nss_database_lookup@GLIBC_2.2.5` exported (compat-only) | binaries from before 2.27 using the `__nss_*` lookups | none | close |
| 18 | MALLOC-COMPAT-5 | 0 ahead | nothing. `tr_break@GLIBC_2.2.5`, `__default_morecore@GLIBC_2.2.5` (compat-only), `__libc_freeres@@GLIBC_2.2.5` exported | old mtrace/morecore users and leak checkers | none | close |
| 19 | COMPAT-CONDVAR-OLD-2 | 0 ahead | nothing. `pthread_cond_init`/`wait@GLIBC_2.2.5` beside `@@GLIBC_2.3.2`; D-22.13 on next | binaries linked before glibc 2.3.2 | none | close |
| 20 | RPC-SERVER-5 | 0 ahead | nothing to port. The server reached the public root, then became failing stubs on purpose (D-29.5, owner rulings R13 and 10-27; LEGACY-RPC-NIS-STUBS-7 landed). `svc_run`, `svctcp_create`, `svcudp_create`, `registerrpc`, `_seterr_reply` are exported `@GLIBC_2.2.5` | old RPC servers: they get the stubs' failure values (ruled) | none | close (obsolete by ruling) |
| 21 | RPC-SERVER-TCP | 0 ahead | as row 20 (the 4 MiB TCP record cap is moot under the stubs) | as row 20 | none | close (obsolete by ruling) |
| 22 | RPC-REPLY-SOURCE | 0 ahead | nothing. Its compat flag COMPAT-12 is withdrawn (`docs/compat-flags.md:17-18`): the UDP client it applied to is a failing stub | nothing | none | close (obsolete by ruling) |
| 23 | RPC-XENCRYPT | 0 ahead | nothing. `xencrypt`/`xdecrypt` exported `@GLIBC_2.2.5` as stubs (D-29.4; "insecure by design", no restore option) | as row 20 | none | close (obsolete by ruling) |
| 24 | RPC-GETRPC-CORPUS | 0 ahead | nothing. The DES/auth part is stubs (D-29.4). `getrpcbyname@@GLIBC_2.2.5` and the /etc/rpc parser are real (`crates/boundary/src/netdb.rs:724`), tested in the s16 suites | nothing | none | close |
| 25 | PERF-HEAP-PAIR-IVB-2 | 0 ahead | nothing. The negative result (K_HINTV, reverted) is in `docs/saturation/14-...md` row 2 | nothing | none | close |
| 26 | GATE-THREADS-REQUIRED | 0 ahead | nothing. `threads-required = 4` in `.config/nextest.toml:91` | nothing | none | close |
| 27 | BENCH-REFRESH-1023 | 0 ahead; ancestor of -4 and PERF-NEXT-BATCH | nothing. A measurement story; sections 11 and 12 of the register carry its -3/-4 results; the numbers are stale by design | nothing | none | close |
| 28 | RELEASE-PUBLIC-TREE-TIDY | 0 ahead; ancestor of -2 and INTEGRATE-REGISTER | nothing. The release-week tidy that the public cut did | nothing | none | close |

## Not verified

- The libresolv.so.2 compat exports of row 16 were not checked by readelf. The product set on hand had none, and
  building one is outside an audit with no jobs. The source path is checked; `crates/core/tests/sonames_products.rs`
  covers the products.
- The libc.so.6 checked by readelf came from a story branch cut from next (11-21), not next itself. Compat exports
  come from compat-plan.tsv, which matches next for every name checked.
