# PLANTS-BUILD-2: one build tag per plants run, per-phase timing in `cargo xtask so`, Zen 1 numbers, concurrency data

State: branch `task/plants-build-2` (stacked on `task/plants-build-slowdown`, base origin/next 255a6901d). Code commits:
0e9852c30 (tag + phase lines), 381293dbb (test pins this process's own tag). Plants file
`handoff/PLANTS-BUILD-2-plants.txt` (12 plants). Report record: `handoff/reports/PLANTS-BUILD-2.json`.

## What changed

1. `cargo xtask plants` sets `NIJI_PLANTS_BUILD_TAG=plants-<pid>` for every command it runs in its copy (`plants.rs`
   `run_build_tag`, `run_logged`). `so::build_tag` uses it when set and non-empty (`tag_or_hash`), else the hash of the
   sources as before. One constant tag per run, so a mutant outside `niji-boundary`/`niji-ld` no longer rebuilds them in
   every product target dir. Products outside a plants copy keep the real tag; D-27.22 and the ld.so start-up check are
   untouched (both products of one build carry the same tag; `reloc_libc_build_tag` still builds a mismatch itself).
2. `cargo xtask so` prints `step: so/<phase> <seconds>s` lines (gate format, `steptime::line`): `headers-gen`,
   `libc-archive` (holds `cargo-libc`), `libc-link`, `crt`, `placeholders`, `libm` (holds `cargo-libm`), `libmvec`,
   `early-builds-wait`, `ld` (holds `cargo-ld`), `ldconfig` (holds `cargo-ldconfig`), `utilities`, `charmaps-placement`,
   `layout`, `total`; `early-cargo-ld`/`early-cargo-ldconfig` are the early builds' own time (they overlap the libc
   phases). Names with a `/` are not summed again by the gate's `step-summary`. `docs/plants.md` says both.

## Numbers

Skylake-SP host (48 threads, RAM copy, `NIJI_LAYOUT_TRACE=ungrouped`; this run's loads 2-4 beside; the
PLANTS-BUILD-SLOWDOWN row for before):

| build (seconds) | origin/next before SLOWDOWN | with this story |
|---|---|---|
| cold `cargo xtask so` | 524 | 160 (real tag), 118 (first build after switching to the constant tag) |
| no-op | 190 | 40 |
| mutant crates/libc-so/src/lib.rs, real hash tag | 344 | 111 (restore 111) |
| same mutant, plants' constant tag | - | 48 (restore 41) |
| mutant crates/boundary/src/hot/hyperbolic.rs, constant tag | 351 (core) | 111 (restore 41) |

Zen 1 host (60 vCPU, but a job runs in a 4-core slot; 6 slots, 24 cores busy; other tenants' load average 8-36, noted per row
in `measure-*.out`; the before and after trees ran at the same time in two slots, 8-26 load). Before = origin/next
255a6901d, after = this story's tree:

| build (seconds) | before | after |
|---|---|---|
| cold | 868 (load 8-11) | 352 (8-10) |
| no-op | 418 (11-13) | 60 (9.5) |
| mutant libc-so/src/lib.rs, real tag | 704 (13) | 236 (9.5-13) |
| same, restore | 746 (13-21) | 234 (11-13) |
| same mutant, constant tag | - | 86 (11.5-12), restore 65 |
| mutant boundary hyperbolic.rs | 717 (21-26) | 230 (constant tag; 12), restore 62 |

The 150 s headers-gen cost is gone on both hosts. The mutants are a one-line comment appended to the file.

### The other option (tag linked from an xtask-built object): what ordinary dev and CI builds would gain

Only an edit that changes the hash but is outside boundary's and ld's dependency cone (csys/, include/, Cargo.lock,
the root crate, the small crates boundary does not depend on): real tag vs constant tag, same mutant: 111 -> 48 s
(Skylake-SP), 236 -> 86 s (Zen 1 slot), and the restore build the same. An edit inside the cone (the usual library
edit) gains nothing: boundary rebuilds anyway (111 s / 230 s). So the gain is 60-150 s on the minority of edits outside
the cone: not large, so no RULING NEEDED; INFORMATIONAL in the report. It would also touch D-27.22 and the loader.

### Concurrent copies (#6763)

Same plants, serial in one `cargo xtask plants` vs two jobs each with half of the plants (two copies, own target dirs):

| host | serial | two copies | peak RAM per copy (cgroup peak, jobs.tsv) |
|---|---|---|---|
| Skylake-SP, 12 plants (serial ran on head f1da2e77c, one plant SURVIVED then) | 1591 s | 860 s (9 plants) and 538 s (3 plants), wall 860 s | 13.6 GB and 9.2 GB (serial 19.8 GB) |
| Zen 1, 4 heavy plants, 4-core slots | 1405 s | 871 s and 857 s, wall from first start to last end 980 s | 10.3 GB each |

Two copies win on both hosts (0.54 and 0.70 of the serial wall; the Skylake-SP halves were unbalanced, so a balanced split
gains more). The default is NOT changed here: it needs `--copies 2` in the tool (split by plants, each copy its own dir
and verdict order kept), which is a story of its own (INFORMATIONAL in the report; recommended default 2).

## Plants

First run (head f1da2e77c): 11 CAUGHT, 1 SURVIVED (`pb2-tag-env-not-set`: a test running under a plants run inherits that
run's tag, so a `run_logged` that set none still passed). Fixed (381293dbb: the test expects this process's own tag).
Final runs on 381293dbb (halves, concurrently): plants-build-2-plants-a 9/9 CAUGHT, plants-build-2-plants-b 3/3 CAUGHT.
Together they are the whole file.

Next step: land; then a story for `--copies 2` in `cargo xtask plants`. Read: `xtask/src/so.rs` `tag_or_hash`, `Phases`;
`xtask/src/plants.rs` `run_build_tag`.
