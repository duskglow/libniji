# LD-LOOKUP-RETHINK: where a symbol lookup's instructions go, and the walk-level Bloom filter

**State:** branch `task/ld-lookup-rethink` from origin/next 099566cb9; code head ae0361b3b (records, handoff and kit
commits on top). Role shiki `~/w/sh2` (Haswell-EP, 26 vCPU). Decision D-27.91 (records/decisions/D-27/). Brief:
~/state/briefs/LD-LOOKUP-RETHINK.md. Kit: handoff/LD-LOOKUP-RETHINK/kit/ (installed as shiki `~/work/ld-lookup-rethink/`).
**Verdict:** design D1 lands (beats next on every row measured here); target 675 per lookup not reached (1,400).
Intel PT on the Skylake-SP host was not available (ssh to mei refused, see the report); attribution used a ptrace
single-step trace of libniji's own release loader instead (exact: every user instruction, totals equal perf's to a
constant 59) with line tables for inlined code; the reference was only counted (perf counters, black box).

## Method (kit: gen2.py, meas2.sh, sstep.c, attr.py, diff.py, phase2.py, cmp.py)
Programs taking N=200 addresses through the GOT (one GLOB_DAT each) minus the same program with none, both loaders,
same libraries (built `-nostdlib`): hit-1st (definer is the first library), past-3 (three filler libraries first),
miss-4 / miss-7 (weak references nothing defines: 4 or 7 objects), hit-ver (one version VA_1), hit-40B / miss-40B
(40-byte names), libc (200 C library functions, GLIBC_* versions). Per lookup = difference / 200. ASLR off.

## Phase split, libniji next 099566cb9, release, instructions per lookup (attr.py + phase2.py)
| phase | hit-1st | past-3 | miss-4 | miss-7 | hit-ver | hit-40B | miss-40B | libc |
|---|---|---|---|---|---|---|---|---|
| relocation decode + dispatch (pass1, one) | 191 | 191 | 191 | 191 | 191 | 191 | 191 | 191 |
| target-address check + store (Loads::inside, put) | 64 | 64 | 65 | 65 | 64 | 64 | 65 | 64 |
| reference symbol, name, request (target) | 192 | 192 | 160 | 160 | 192 | 252 | 220 | 205 |
| name hash (GNU, hashed_string) | 76 | 76 | 76 | 76 | 76 | 272 | 272 | 82 |
| versions, both sides (memo, reference_want_origin) | 45 | 45 | 16 | 16 | 125 | 45 | 16 | 271 |
| scope walk (loop, objects.get, Result) | 151 | 250 | 200 | 299 | 151 | 151 | 200 | 151 |
| per-object filter (call, Bloom, bucket, chain) | 230 | 449 | 299 | 518 | 219 | 230 | 306 | 221 |
| candidate check (symbol, kind, compare, versym) | 157 | 157 | 0 | 0 | 239 | 172 | 0 | 255 |
| other | 3 | 3 | 5 | 5 | 9 | 3 | 6 | 10 |
| **total** | **1,109** | **1,427** | **1,012** | **1,330** | **1,267** | **1,380** | **1,277** | **1,449** |
| glibc 2.39 (black box, same programs) | 553 | 681 | 444 | 572 | 639 | 800 | 635 | 675 |

Black-box model (both loaders, same fit): per object walked past libniji 106 / reference 43 ((past-3 - hit-1st)/3,
(miss-7 - miss-4)/3 agree); fixed per lookup (miss-4 less 4 objects) 588 / 273; hit work in the definer 414 / 237;
one version +158 / +86; per name byte (misses: hash only) 8.3 / 6.0. Hashing is once per lookup (76 for 8 bytes in
every scenario). The libc column's versions (271 against 125 for one version) are memo misses: the 64-slot memo is
direct-mapped by `(index, side, string-table address)`; memo_fill is 90 per lookup inclusive there, 4.2k on an
empty main, 7.3k on `/bin/true`. The slots depend on where the tables are mapped, so with ASLR on the miss count can
change from start to start (not measured: the kit runs ASLR off).

## Why LD-LOOKUP-STRUCTURE measured +34 per lookup (explained from the split; its binary was not rebuilt here)
Its table replaced the version work for recorded requirements, expecting -90. The split bounds that work at 45 per
lookup for an unversioned definer and 125 for one version; the memo hit path inside it is about 20-40, so -90 was
reachable only through the memo misses (the libc column), and an 8-slot table from the lowest requirement index
leaves most of a program's GLIBC_* versions on the memo path behind a failed probe. Everything else a lookup pays
(decode, request, hash, walk, candidate check: over 1,000 of 1,109) is untouched by a version table, while its
probe and the table threaded through pass1 -> one -> target run on every relocation: a net rise is the expected
sign. NOT VERIFIED: the exact +34 (rules allow only its handoff note; its code was not built or traced).

## Designs (estimates from the split; per lookup unless said)
| design | phase it removes | estimate | status |
|---|---|---|---|
| D1 walk-level Bloom test, object entered only on a pass | per-object entry (~56 of 106 per object walked past) | -50 per object; empty main -3k, 3 libs -15k | **prototyped, measured, lands** |
| D2 version names by index per object (built at load), no hashed memo | versions 271 -> ~60 on libc names; ASLR-independent | -150..-200 (libc names); empty main -3..-5k, `/bin/true` -6..-8k | brief: handoff/LD-LOOKUP-RETHINK/LD-VERSION-INDEX.md |
| D3 flat relocation path over a view validated at load | decode + store + request + hash = 523 (reference whole lookup 553) | -320..-370; empty main -19..-22k, `/bin/true` -33..-38k | brief: handoff/LD-LOOKUP-RETHINK/LD-RELOC-FLAT.md |
| D4 per-start (name, version) answer cache | repeats only | ~0 on start-up rows (few repeated names), + insert cost | rejected |
| D5 lazy binding of JUMP_SLOT | lookups deferred to first call | 0 measured: `/bin/true` is BIND_NOW (34 JUMP_SLOT + 9 GLOB_DAT bound at start by both loaders) | rejected |
| D6 word-at-a-time GNU hash | hash 76 (8 bytes), 272 (40 bytes) | -40 (C names), -150+ (C++ names) | small story, after D3 |
D1 + D2 + D3 + D6 reach about 800-860 on libc names; the rest is the candidate check (157 against the reference's
whole hit work of 237) and the walk loop (50 per object against 43).

## Results (release, shiki, user instructions, median of 21, ASLR off; faults unchanged 61 / 73 / 59)
| row | next 099566cb9 | D1 first form (fb75b1e12) | **D1b (ae0361b3b)** | glibc 2.39 |
|---|---|---|---|---|
| empty main | 204,525 | 203,528 | **201,801 (-2,724)** | 99,928 |
| three libraries | 307,109 | 294,049 | **291,992 (-15,117)** | 135,522 |
| `/bin/true` | 247,585 | 245,924 | **242,991 (-4,594)** | 124,662 |
| per lookup, libc names | 1,449.4 | 1,433.2 | **1,399.5 (-49.8)** | 674.7 |
| per lookup hit-1st / past-3 / miss-4 / miss-7 | 1,109 / 1,427 / 1,012 / 1,330 | 1,097 / 1,249 / 799 / 948 | **1,059 / 1,211 / 796 / 945** | 553 / 681 / 444 / 572 |
The first form instantiated the chain walk twice (outlined: the definer paid a call), so a hit gained 12 not 59.
Only one CPU class measured (Haswell-EP): Zen 1 and Skylake-SP not run here (loader instruction counts differed by
0.05% between those classes in STARTUP-REPROFILE). Not timed (instructions to work).

## Jobs (shiki)
Final head: ld-lookup-rethink-d1b (release build, counts, traces, clippy: rc 0), ld-lookup-rethink-plants (5 plants:
4 CAUGHT, 1 EQUIVALENT, no WARN; handoff/LD-LOOKUP-RETHINK-plants.txt), ld-lookup-rethink-gate (direct tier, 35
targets), ld-lookup-rethink-startup (cargo xtask test; release products + hot_text_products). Evidence on base:
ld-lookup-rethink-so-base, -meas-base2 (counts; its trace step failed on a kit bug), -trace-base. Superseded:
-meas-base (kit bug), -d1 (first form). Results of gate/startup: see the report json.

## Next step
Land D1b. Coordinator: brief LD-VERSION-INDEX and LD-RELOC-FLAT (files above); count D1b on Zen 1 / Skylake-SP.
