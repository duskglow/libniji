# configure-diff: every build-system probe that answers differently on libniji than on stock

CONFIGURE-DIFF's rerunnable tool. For each Debian source package it runs the package's own configure step
(Debian flags, overrides and all) three times in one throw-away trixie chroot, same source, same directory path:
`stockA` and `stockB` on trixie's libc (the stock-vs-stock control), then `niji` after libniji's system flavour
(`libc6`, `libc-bin`, `libc6-dev`, `libc-dev-bin`) is installed over it. Every probe answer is extracted and the
three runs are compared per probe.

## Files

| File | Runs | What |
|---|---|---|
| `cdiff.sh` | build host | driver: base chroot tarball (once per run dir), N packages at a time, then extract and diff |
| `pkg.sh` | build host | one package: `mmdebstrap --mode=unshare` from the base tarball, runs `inside.sh`, copies the results out |
| `inside.sh` | chroot | build-dep, source, configure stockA/stockB, swap libc, configure niji; tars the probe files |
| `cfgrules.sh` | chroot | `dpkg-buildpackage -T` rules file: the dh `build` sequence up to and including the configure step |
| `extract.py` | build host | probe answers per side: autoconf cache and output variables, generated headers, perl `config.sh`, meson checks, CMake cache |
| `diff.py` | build host | per package probes compared / noisy (stockA != stockB) / differing (stock agrees with itself, libniji differs) |
| `why.py` | build host | the config.log (or meson-log) blocks of one probe on both sides: command, compiler messages, test program |
| `mklist.sh` | build host | the package list from Aqours' stage-2 `*.result` files |
| `pilot.list` | | a 10-package set with the known gaps (the 1127B ybuild set, perl, apt, glib2.0, gcc-14) |

gcc-14 is special: only libstdc++-v3's configure runs, standalone from the upstream tarball, with the system
`g++-14 -nostdinc++` (so `<math.h>` is the C library's, as in gcc's own build); this is not Debian's gcc build.

## Run

One job per package set, on a build host with `mmdebstrap` and a trixie mirror (the mirror URL and keyring are
arguments, never in the tree); run dir in RAM:

    sh cdiff.sh -m <trixie mirror url> -k <debian archive keyring> -d <dir with libniji's system debs> \
        -o /mnt/zram/claude/<story>/<run> -j 20 <pkglist>
    python3 diff.py <run> --sides stockA,stockB     # the stock-vs-stock control: expect differing=0

Outputs in the run dir: `summary.txt` (per package), `diff.tsv` (every differing probe, with a first-guess class),
`noise.tsv` (probes that differ stock vs stock: never counted), `probes/<pkg>.<side>.tsv`, `pkgs/<pkg>/out/`
(status, build-dep, source and configure logs, the probe files). A later snapshot story reruns it with its own
debs and expects 0 libniji-class rows in `diff.tsv` beyond the recorded differences listed in
`handoff/CONFIGURE-DIFF-report.md`.

## Known limits

* A package whose `debian/rules` does not use dh runs the shortest rules target whose name says configure; one
  without such a target (a plain Makefile package such as keyutils) has no probes.
* Devuan-versioned Aqours packages (`...devuan1`) are not on the Debian mirror: trixie's version runs instead
  (`status.txt` `version=trixie`).
* The stock side is trixie's libc (2.41); differences that come from 2.41 versus libniji's 2.39 base are classed
  as such in the report. With `-s` the tool classes them itself (below).

## Two stock sides (`-s`, RELEASE-SNAPSHOT-1127C)

    sh cdiff.sh ... -s <dir with stock 2.39's libc6, libc-bin, libc6-dev, libc-dev-bin .debs> ... <pkglist>

The Ubuntu noble 2.39 debs (`apt-get download libc6=<v> libc-bin=<v> libc6-dev=<v> libc-dev-bin=<v>` on a noble
host; the version goes in the report) are installed over libniji's after the niji run, in the same chroot, and the
configure step runs a fourth time as `s239`. The niji side is unchanged by `-s` (it still runs right after stock
2.41), so a two-side run's niji answers compare with a one-side run's. The s239 side runs only when that swap
succeeded with all four packages at a `2.39-0ubuntu` version (`status.txt`: `swap239`, else `s239 SKIPPED`).
`diff.tsv` then has two more columns, `stock239` and `vs`: `libniji-vs-both` (libniji differs from 2.41 and from
2.39: libniji's own difference), `2.41-only` (libniji answers as 2.39: a 2.41 change), `no-s239` (not classed).
`as241.tsv` lists the probes where libniji answers as 2.41 and 2.39 differs (2.41 behaviour libniji already has,
e.g. the recorded `sched_attr` family). `summary.txt`'s last line counts each class. There is no stock-vs-stock
control for the 2.39 side itself; the stockA/stockB control still filters non-deterministic probes.
