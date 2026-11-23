# CI environment audit: what the gate reads besides the code (CI-ENV-AUDIT, 2026-11-23)

Audit only: nothing in the CI was changed. Hosts are named by CPU class: the **CI host** (Skylake-SP
Xeon Platinum 8151, 48 vCPU, runs the landing gate), the **Zen 1 host** (EPYC 7551, 60 vCPU, runs
the CI's instruction counts and developer gates) and two more developer build hosts, **Haswell**
(Xeon E5-2697 v3) and **Broadwell** (Xeon E5-2699 v4).

## Method

* **Red history**: every CI gate log from 11-13T02:39 to 11-23T16:03 (300 gates: 251 green, 49 red),
  grouped by failure signature (`agg.py` below), each signature read down to its panic message.
* **Workspace**: the CI role's `target/` listed by size, age and name on the CI host (read-only).
* **Hosts**: one manifest (`manifest.sh`, the prototype of proposal 1) taken on all four build hosts
  and compared key by key.
* **Traces**: `strace -f` (execve, open/openat/stat/access/readlink/mkdir/unlink/rename families) of
  `cargo xtask so` and of `cargo xtask affected --only <the product suites that went red> --run` on
  a role of the Zen 1 host, summarised to host paths outside the role and the toolchain home
  (section "Traces").
* **Cost**: one role emptied and rebuilt (cold), then rebuilt with no change, with only
  `target/niji` and `target/niji-test` removed, and with one source file touched (section
  "Workspace question").
* **Code reading** only to explain what a trace or a red showed (cache keys, test scratch paths).

## Inventory and classification

Class: **hermetic** = pinned or rebuilt every gate; **checked** = verified, with a clear failure
naming the input; **ambient** = assumed. "Seen" = how this audit observed it.

| # | Input | Seen | When it differs from the CI host's | When absent | Class |
|---|---|---|---|---|---|
| H1 | Locales of the host (`locales`, `locales-all`; `/usr/lib/locale`) | manifest; trace | CI host: `locales-all` 2.39-0ubuntu8.**12** (509 locale directories, no `locale-archive`, 511 names) beside `libc6` 2.39-0ubuntu8.**9**; the three developer hosts: one `locale-archive`, 4 names. Installing `locales-all` on the CI host failed 7 locale and catalog tests in every gate (11-23 07:07, 07:29). A developer gate can never reproduce a CI locale red | locale tests compare against the reference's behaviour for the host's locales; a missing locale is a skip or a mismatch | ambient |
| H2 | Reference C library (`libc6`, loader, `libc6-dev`, `/usr/include`) | manifest; trace (oracles link it) | same package version on all four hosts today (2.39-0ubuntu8.9); nothing notices a change | every `*_equals_the_reference` test and oracle fails to build | ambient |
| H3 | Host tools: `cc`/`gcc`/`g++` 13.3, binutils 2.42, `ld.lld`, `localedef`, `gencat`, `msgfmt`, `iconv`, `zic`, `readelf`, `objcopy`, `dpkg-deb`, `python3`, `perf`, `sudo -n`, `gdb` (also used to lay out the products) | manifest; trace (execve list) | identical on the four hosts today; no check | most tests skip with `SKIP:` (TEST-SILENT-SKIPS prints them); C builds fail | ambient |
| H4 | Kernel UAPI headers (`linux-libc-dev`, copied by headers staging) | code (`xtask/src/headers.rs` UAPI list) | staged into the sysroot as found | that header family is skipped | ambient |
| H5 | Time-zone data, `/etc/{hosts,resolv.conf,nsswitch.conf,services,protocols,localtime}` | manifest; trace | `/etc/hosts` differs on every host; `nsswitch.conf` differs on two; tzdata 2026e on all | name-service tests compare with the reference on the same host, so mostly self-consistent | ambient |
| H6 | Kernel release | manifest | CI host 6.8.0-**149**; the three developer hosts 6.8.0-**147** (CI host upgraded 10-23 build; nothing recorded it) | — | ambient |
| H7 | CPU model and flags (dispatch, verdict-cache key) | manifest | CI host has AVX-512 and ERMS; the developer hosts do not: CPU-dispatched paths the CI runs are never run by a developer gate | — | ambient (recorded in the verdict-cache key only) |
| H8 | Sysctls and limits (`randomize_va_space`, `overcommit_memory`, `max_map_count`, `ptrace_scope`, THP, `ulimit -s/-n`) | manifest | identical on the four hosts today | heap, `RLIMIT_AS` and spawn tests depend on them | ambient |
| T1 | Rust toolchain (`rust-toolchain.toml`: 1.99.1, clippy, rustfmt) | trace; manifest | pinned by the tree; rustup installs it | rustup fetches it (network) | hermetic |
| T2 | `cargo-nextest` | manifest | CI host 0.9.148, Zen 1 and Haswell 0.9.146, Broadwell 0.9.149 | `cargo test` fallback (`NIJI_NO_NEXTEST`) | ambient |
| T3 | User cargo config `~/.cargo/config.toml` | manifest | CI host: `[profile.dev] debug = 0`; Zen 1 host adds `[build] incremental = false`; three different files on four hosts | cargo defaults (debuginfo on: larger, slower builds) | ambient |
| T4 | Pinned extra toolchains for `rust-dylib` (`spec/abi/rust-abi.toml`) | code; manifest | same toolchain set on the four hosts | `rust-dylib` fails naming the toolchain | checked |
| W1 | Cargo build cache (`target/debug`, `target/release`, `target/product`) | workspace listing; cost runs | fingerprinted by cargo (rustc version, flags, sources) | rebuilt: cost in "Workspace question" | hermetic (content and mtime keyed) |
| W2 | Products `target/niji`, `target/niji-test` | workspace listing; red history | stale products from earlier trains failed the manifest lint (`gencat`, `niji-unwinder.o`) and the SHSTK check (`rust-test/lr999-test`, `rust/lr1`) for whole trains | rebuilt | hermetic since 11-23 (the CI now removes both before each gate) |
| W3 | Test oracles `target/test-oracles` (902 files, 1.9 GB) | code (`support/oracle.rs`); listing | rebuilt when the command line or a named input or header beside it changes; **not** when the host compiler, `libc6-dev` or `/usr/include` changes | built on first use | ambient for host inputs |
| W4 | Test locales `target/test-locales*` (built 11-10 with the host's `localedef`) | code (`oracle::locale`); listing | **never rebuilt**: existence of one probe file is the whole key; a new `localedef` or `/usr/share/i18n` source is never seen | built on first use | ambient |
| W5 | Verdict caches `target/test-verdicts` (D-10.57) and `target/layout-verdicts` | code | keys hold the CPU flags, kernel release, host libc and loader, and the inputs' contents | the check runs | hermetic |
| W6 | Test scratch left under `target/` | listing | 631 per-process directories, 2.4 GB, oldest 11-10: `ld-runpath-*` 4 x 52 (one set per train since 11-21, about 27 MB a train), `ld-names-*` 4 x 30, `dl-search-*` 286, and the dirs of failed runs | — | ambient (grows the RAM disk; it reached 91-93 %) |
| W7 | Pre-rename products `target/liella`, `target/liella-test` (124 MB) | listing | read by nothing today; any check that walks `target/` would see them | — | ambient |
| E1 | Environment of a gated job | `~/bin/gate`; trace | reset to `HOME`, `PATH`, `SLOT_HELD` (`PATH` starts with `~/.cargo/bin` and `~/bin`, ahead of `/usr/local/bin` and `/usr/bin`: a stray `~/bin/cc` or `/usr/local/bin/ld` replaces the host's in every gate); xtask then sets `NEXTEST_TEST_THREADS`, `NIJI_TEST_MALLOC_PERTURB`, `NIJI_SKIP_LOG`, the pass-2 runner; `CC`, `AR`, `LD`, `OBJCOPY`, `RUSTFLAGS`, `CARGO_TARGET_DIR`, `TMPDIR` are honoured if set (none set by the wrapper) | — | hermetic (wrapper) |
| E2 | `/tmp`, `/dev/shm`, `/dev/mqueue` | `~/bin/gate` | private tmpfs per gate (fresh, empty) | — | hermetic |
| E3 | Gate wrapper and job tools (`gate`, `jobrun`, `shared`, `slots`, `static-checks`) | md5 on the four hosts | identical on three hosts; Broadwell host's `jobwait` and `slots` differ | — | ambient (deployed by hand) |
| E4 | Memory limit (`MemoryMax` of the gate's scope) and the slot's cores | `~/bin/gate`, `slots-status` | a developer gate runs in a 4-core slot; the CI gate uses more (`NEXTEST_TEST_THREADS=24`) | — | checked (`oom_kill=` printed) |
| D1 | RAM-disk space (CI role on `/mnt/zram`, 126 GB) | `df` | 50 % now; 91-93 % on 11-23; the CI role alone holds 17 GB, 2.4 GB of it W6 | builds fail with ENOSPC mid-step | ambient (watchdog alert only) |
| D2 | Target file system | `df` | CI: ext4 on zram; Zen 1 host: tmpfs (`/srv/ramtarget`); others: ext4 on SSD. Page-fault bounds of start-up tests read programs from these | — | ambient |

## The reds

49 red gates in 300 (11-13 to 11-23). Classes: **H** host state, **W** workspace state, **B** a
start-up bound missed by a small margin (owner ruling #4347 treats a start-up fault red on a change
that touches no start-up code as layout jitter), **R** recurring across consecutive trains with
different heads, cause not settled here (a defect on `next` or the host: this audit could not tell
which), **S** the train's own code (compile errors, a story's test, generated views not
regenerated). S reds are not counted below.

The last 20 non-story reds, newest first:

| # | Gate (head) | When (UTC) | Signature | Class | Evidence |
|---|---|---|---|---|---|
| 1 | 29657fdab | 11-23 16:03 | `init_dynamic_empty_main` 173794 instructions > 173600 | B? | 0.11 % over; no start-up story identified |
| 2 | 6f9fd5047 | 11-23 15:36 | release start-up check: a phase program exits 99 | B/H | RELEASE-STEP-EXEC-FAIL owns the cause |
| 3 | 0fabf6a8f | 11-23 14:51 | release start-up check: page faults over the bound | B | same check, other test |
| 4 | 0d881a849 | 11-23 14:02 | 17 objects lack SHSTK: `rust-test/lr999-test/...`, `rust/lr1/...` | W | products of an earlier train under `target/niji` |
| 5 | da654d8aa | 11-23 13:44 | `deb_manifest` names unbuilt products | W | stale `gencat`, `niji-unwinder.o` |
| 6 | b154c509c | 11-23 13:10 | `deb_manifest` | W | same |
| 7 | a176ecda8 | 11-23 07:29 | 7 locale/catalog tests + `deb_manifest` | H+W | `locales-all` installed on the CI host |
| 8 | 691c188e3 | 11-23 07:07 | 7 locale/catalog tests | H | same |
| 9-13 | d5421bf93, ff5ae1612, 74021ce35, 0903a6ffc, 521cdc7e1 | 11-21 22:54 to 11-22 07:32 | `heap_meta_products`: the program fails under `RLIMIT_AS` = start VmSize + 41296 KiB | R | 5 trains in a row; the headroom is measured over the start VmSize, which the host's kernel and limits shape |
| 14-16 | 5ace8cb22, da313127b, 08084029d | 11-21 18:09 to 21:46 | `dns_products resolv_equals_the_reference` | R | 3 trains in a row; cause not investigated by this audit |
| 17 | c2167be9f | 11-20 01:08 | release start-up: 68 page faults > 67 | B | one fault over |
| 18-20 | c7b254cda, 4be367254, d74184ea0 | 11-19 08:26 to 10:02 | `aio_engine`: the test's scratch file `NotFound` | R | 3 trains in a row |

Beyond the window: d726fdaa1 (11-19, B, 87 > 86 faults), 1e641edc8 (11-18, B, 91 > 89),
f7d1388f7 and b4bfd4979 (11-17, H: conformance `s18_spawn_vm` fails against the **reference**, so
the host, not libniji), 821416660 (11-16, W: `native_static: left ["sub"] in target/niji/native/run`).

S reds in the same period, for scale: headers-gen-check not regenerated (4), `niji-ld`
dlclose/atexit assertions (2), `fast32` fallback rate (2), compile error (1), `cpow` (1), and the
11-13 to 11-15 layout-contract and start-up-list reds of stories in flight.

## Workspace question

Does the CI role need state across trains beyond cargo's build cache? **No**, by measurement:

Zen 1 host, one 4-core slot, target on tmpfs, `fast` products (the gate's default), head 0495c4684:

| Phase | `cargo xtask so` | `cargo test --workspace --no-run` | Total | `target/` after |
|---|---|---|---|---|
| cold (target emptied) | 674 s | 505 s | 1179 s | 1.44 GB (debug 0.81, product 0.56, niji 0.06) |
| no change | 302 s (`so/libc-archive` 240 s; 0 crates compiled) | 0.6 s | 303 s | 1.44 GB |
| only `target/niji`, `target/niji-test` removed | 302 s | 0.6 s | 302 s | 1.44 GB |
| one source file touched (`crates/core/src/ctype.rs`) | 548 s (`so/libc-archive` 377 s) | 378 s | 926 s | 1.44 GB |

* Removing the products before each gate (what the CI does since 11-23) costs **nothing measurable**
  (302 s against 302 s): `so` re-archives and relinks every product on every run anyway.
* A from-scratch build costs 253 s (27 %) more than a train that changes one file of `niji-core` on
  this slot (1179 s against 926 s): `niji-core` is below every test binary, so most of them rebuild
  anyway. A CI train changes hundreds of files (336 in the 16:03 train), which puts the CI's
  incremental build closer still to cold. (This host's `~/.cargo/config.toml` turns cargo's
  incremental compilation off; the CI host's does not, input T3, so the CI's one-file case is
  somewhat cheaper than measured here.)
  CI gates (48 vCPU) spent 136-329 s in `so/total` and 360-1006 s in `test/tests-nextest` over the
  last 40 gates.
* Disk: a from-scratch fast-profile target is 1.44 GB; the CI role's target is **17 GB** (debug
  8.0, product 2.4, test-oracles 1.9, crt-products 1.3, float-casts 0.5, release 0.3, W6 leftovers
  2.4). Everything but cargo's directories and the content-keyed caches (W5) can go before each
  gate; the oracles (W3) and test locales (W4) are cheap to rebuild only if keyed correctly
  (proposal 4).

## Traces

Job `ci-env-audit-trace` on the Zen 1 host, under `~/bin/gate` like a gate: `trace.sh` above,
summarised by `summ.py`. Two limits: `cargo xtask so` stops under strace at its trace-at-build step
(that step runs the products under gdb, which cannot attach to a traced process), so the build trace
covers the steps before it (headers, the early cargo builds); and the product suites ran on the
products of the cost run, where the `hot_text_products` start-up tests fail under ptrace (11 of 109
tests: counts and gdb change under a tracer). Both are diagnostics, not verdicts.

Environment inside the gate: `HOME LOGNAME OLDPWD PATH PWD SHELL SLOT_HELD TERM USER` only.

Programs the traced test run executed (successful execve, by count): `readelf` 3193, `objcopy`
2952, `cp` 456, `cc` 433, `as` 408, `ar` 132, `nm` 111, `ld` 87, `timeout` 77, `gzip` 71, `iconv`
70, **`gdb` 69**, `localedef` 54, `gcc` 37, **`perf` 33**, `dpkg-deb` 21, `strip` 16, `tar` 15,
**`sudo` 8**, `git` 4, `python3` 4, `objdump` 4, `ld.lld` and `rust-lld` 9 and 8, plus the test
programs. Each is a host input (H3); `gdb` is also an input of the products themselves (the
trace-at-build step lays out the hot text from a gdb trace), and `perf` reads
`kernel.perf_event_paranoid` and `/sys/bus/event_source`.

Host paths read outside the role and the toolchain home (successful / failed accesses):

| Path group | ok | failed | What it is |
|---|---|---|---|
| `/sys/devices/system` | 21256 | 0 | CPU topology and caches (dispatch, `nproc`) |
| `/usr/bin`, `/lib/x86_64-linux-gnu`, `/usr/lib/gcc`, `/usr/include/...` | 19861, 15715, 2803, 5863 | — | H2, H3 |
| `/usr/local` | 258 | 25382 | PATH and header search: **a file placed under `/usr/local/{bin,include,lib}` would shadow the distribution's** |
| `~/bin/<tool>` (`cc`, `ld`, `nm`, `gdb`, `strip`, `gcc`, `ar`, `env`, ...) | 0 | ~1500 | `~/bin` is first on the gate's `PATH`: **a program named like a host tool dropped in `~/bin` replaces it in every gate** (E1) |
| `/etc/ld.so.preload` | 0 | 3900 | absent today; if it ever exists, every reference program changes |
| `/etc/ld.so.cache` | 4031 | 0 | H2 |
| `/usr/lib/locale`, `/usr/share/i18n`, `/usr/share/locale`, `/etc/locale.alias` | 414, 1211, 120, 6 | 1166, 24, 57, 0 | H1 (the failed ones are locale names probed and absent on this host) |
| `/etc/localtime`, `/usr/share/zoneinfo` | 92, 16 | 0 | H5 |
| `/etc/nsswitch.conf`, `/etc/passwd`, `/etc/group` | 26, 11, 2 | 0 | H5 |
| `/usr/share/gdb`, `/etc/gdb`, `/usr/lib/python3.12` | 3076, 49, 4648 | 966, 0, 294 | gdb's own data and Python (H3) |
| `/sys/fs/cgroup`, `/proc/<pid>/{maps,cgroup}`, `/proc/sys` | 368, 421, 224 | 361 | limits and layout (E4, H8) |
| `/tmp/<scratch>` | 3556 | 2291 | the gate's private `/tmp` (E2) |
| `~/.gitconfig`, `/etc/gitconfig`, `~/.gdbinit`, `~/.config/gdb`, `~/.perfconfig`, `/etc/perfconfig` | 0 | 41, 9, 25, 26, 10, 18 | absent today: **user and system config files these tools would read if they existed** |

Write accesses outside the role: only the gate's `/tmp` and `/dev/null`, and the RAM target.

## Host drift manifest (specification; the coordinator builds it in liella-ops)

1. **What**: `manifest.sh` (prototype under "Files" below) prints sorted `key<TAB>value` lines: kernel
   release and version; CPU model, count, a hash of the flags line and named flags (avx2, avx512f,
   avx512bw, bmi2, erms, fsrm, shstk, ibt, user_shstk); OS release; reference libc version string and
   the sha256 of `libc.so.6` and the loader; first version line of each tool (`cc gcc g++ ld ld.bfd
   ld.gold as ar objdump readelf nm python3 make strace gdb msgfmt gencat localedef iconv zic` and
   `cargo-nextest`); rust toolchains installed; sha256 of `~/.cargo/config.toml`; locale archive hash
   and size, number of locale directories, hash of `locale -a`, hash of the `/usr/share/i18n` locale
   and charmap lists; tzdata version; gconv cache hash; hash of each of `/etc/{resolv.conf, hosts,
   nsswitch.conf, services, protocols, localtime, ld.so.preload, ld.so.cache}`; package count and a
   hash of all `package=version` pairs, plus named package versions (`libc6 libc6-dev libc-bin
   locales locales-all tzdata gcc g++ binutils libstdc++6 libgcc-s1 gettext linux-libc-dev`); the
   sysctls and THP settings of H8; `ulimit -s -n -v`; file-system type of `/tmp`, home and the role.
   Added from the traces (not yet in the prototype): the list of `~/bin` and of `/usr/local/{bin,
   include,lib}`, `gdb`/`perf`/`ld.lld`/`dpkg-deb` versions, and the presence of `~/.gitconfig`,
   `~/.gdbinit`, `~/.config/gdb`, `~/.perfconfig`, `/etc/perfconfig`, `/etc/malloc.conf`.
   About 100 lines, under 2 s, no root.
2. **Baseline**: a reviewed file per CI host (`<host>.manifest`) in liella-ops, updated only by a
   host-change step (proposal 2).
3. **When**: at the start of every CI gate, before the build, and in the watchdog's 5-minute pass.
4. **Verdict**: a differing key ends the gate before it runs with `HOST DRIFT: <key>: <baseline> ->
   <now>` and a distinct verdict (`host`, not `red`): no story is blamed, no story is split off, the
   train waits for the coordinator. Keys that may drift without review (the `/etc/hosts` hash, CPU
   count) are listed as `advisory` in the baseline and only printed.
5. **Developer hosts**: the same script prints the diff against the CI baseline at the top of every
   developer gate (advisory), so a developer reading a green gate on a host without `locales-all`
   or AVX-512 knows which CI inputs the gate did not have.

## Proposals, ranked by the reds of the window each would have prevented

| Rank | Proposal | Prevents (of the 20) | Also |
|---|---|---|---|
| 1 | **Workspace allow-list**: before each gate, remove everything in `target/` except cargo's own directories (`debug`, `release`, `product`, `.rustc_info.json`, `CACHEDIR.TAG`) and the content-keyed verdict caches (W5). Extends the 11-23 removal of `target/niji*` to W3, W4, W6, W7 | 4 (#4-7; and 821416660 before the window) | the 2.4 GB of W6 and the RAM-disk alerts; measured cost of the product part: 0 s |
| 2 | **Host changes are steps, not events**: hold every package on the CI host (`apt-mark hold`, unattended upgrades off: root, owner's), and change the host only through a "host change" story that records the new manifest baseline and runs a gate of `next` before and after | 3 (#2 if RELEASE-STEP-EXEC-FAIL finds a host cause; #7, #8) | the 11-17 `s18_spawn_vm` reds |
| 3 | **Host drift manifest** at gate start (above) | turns #7, #8 into one named `HOST DRIFT: pkg.locales-all` before the build, instead of 2 trains and 7 tests each | kernel 149 vs 147 and CPU differences visible to developers |
| 4 | **Host-keyed test caches**: the oracle stamp (W3) also hashes `cc --version`, the reference `libc.so.6` and `libc6-dev`'s version; the test-locale key (W4) holds `localedef --version` and the source file's hash | 0 in the window | removes two stale-forever caches; a host change then rebuilds instead of reusing |
| 5 | **Base triage on recurrence**: when a red signature repeats in a second consecutive train, gate `next` alone on the same host at once; a red `next` is a base defect or a host change, never the train's | 0 prevented; settles the 11 R reds (#9-16, #18-20) after one extra gate instead of 3-5 trains | the 3-5 trains each R signature cost |
| 6 | **Start-up bounds judged against the same run's base**: build `next`'s products in the same gate and compare the train's counts with them (the delta, not only the absolute bound) | 3-4 B reds (#1?, #3, #17; #2 if it is a bound) | RULING NEEDED: changes how start-up bounds gate (ALWAYS FORWARD: no bound is loosened; the absolute bound still holds when next itself is within it) |
| 7 | **Pin the remaining toolchain**: `cargo-nextest` version in the tree (checked at gate start), one `~/.cargo/config.toml` (or its settings moved into the tree's `.cargo/config.toml`); the gate's `PATH` without `~/bin` and `/usr/local/bin` (the job tools are called by full path) | 0 in the window | developer and CI builds become the same build; a stray tool cannot shadow the host's |
| 8 | **Hermetic root (longer term)**: run the gate in a pinned root file system image (an Ubuntu 24.04 snapshot with the CI's package set, entered with `unshare -r -m` and a chroot or bind mounts), so H1-H5 come from the image, not the host | everything H: 3 (#2?, #7, #8) | developer hosts run the identical image; only the kernel and CPU stay ambient (H6, H7: recorded by the manifest) |

## Files

The scripts this audit ran (copies here; they ran from a scratch directory on each host):
`manifest.sh` (host manifest), `agg.py` and `sig.py` (red signatures of the CI gate logs),
`steps.py` (build step times), `measure.sh` (the cost runs), `trace.sh` and `summ.py` (traces and
their summary). Manifests of the four hosts: `manifest-ci.tsv`, `manifest-zen1.tsv`,
`manifest-haswell.tsv`, `manifest-broadwell.tsv`.
