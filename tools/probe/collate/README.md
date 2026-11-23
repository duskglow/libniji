# Collation probes (LOCALE-COLLATE-TAILORINGS)

Black-box probes of the reference's `strcoll`/`strxfrm`/`wcscoll` per locale, used to derive the collation order
libniji's generated `LC_COLLATE` sections follow (coordinator decision #7890, owner ruling #7791: locale data by
black-box probe). Nothing here reads the reference's code or its locale source files: the locales are compiled with the
installed reference `localedef` from the installed data and only the compiled result is called through the C API.
Results and findings: `results/locale-collate-2026-11-23.md`.

| file | what it does |
|------|--------------|
| `collprobe.c` | Sorts short word lists per locale (sv_SE, es_ES, tr_TR, pl_PL, zh_CN, ja_JP, ko_KR) and prints whether the order is the tailored one, the root one or byte order; prints `setlocale`'s return so a missing locale (fallback to C, byte order) is visible. |
| `collorder.c` | `collorder <locale> <lo> <hi>...`: every code point in the ranges, sorted by `strxfrm` key, one line `U+XXXX<TAB>level` per element: the first level (1, 2, 3, 4) at which it differs from the previous line, `=` when `strcoll` returns 0, `5` when it differs beyond level 4 (in practice an expansion next to it, or the tie-break between undefined classes; see the results page). The level is found with filler suffixes (method in the source comment). stderr: counts, filler checks, case-first, secondary direction, and `strcoll`/`strxfrm`/`wcscoll` consistency. |
| `collcontr.c` | `collcontr <locale> <cp>...`: two-character contractions among the candidates: for each first character x, the strings x+y must sort as the y do; the ones out of order (not on a longest increasing subsequence) are printed. |
| `colldiff.py` | Expresses one `collorder` output as rules against a root output: moved elements (`&anchor level cp`) off a longest common subsequence, and level changes of kept elements. |
| `ducet_order.py` | Orders the probed code point set by the DUCET (`spec/unicode/15.1.0/allkeys.txt`, UTS #10, implicit Han weights per UTS #10 section 10.1) for comparison with the probed root. |
| `eqruns.py` | Summarises runs of `strcoll`-equal elements (the undefined classes). |
| `build_locs.sh` | Builds named locales with the reference `localedef` into a private `LOCPATH` directory. |
| `survey_all.sh`, `survey_one.sh` | The survey: every UTF-8 locale the reference lists as supported, built in a RAM directory, probed with `collorder` over U+0020-U+D7FF, U+F900-U+FFFD, U+10000-U+1FFFF, built locale removed; records `localedef` time and the compiled `LC_COLLATE` size. `LDTIMEOUT` (seconds, default 300) bounds `localedef`. |
| `diff_all.sh`, `diff_vs.sh` | `colldiff` of every surveyed locale against a root (en_US), or of named locales against another base (en_CA for the upper-case-first locales); `groups.tsv` groups identical orders. |
| `contr_one.sh`, `contr_run.sh` | Contraction pass: Latin candidates (U+0041-U+017F letters) plus the code points the locale moves against the root, when at most 300. |

Running (on a build host, as a `jobrun` job; `<w>` is a work directory holding these files and the compiled
programs, `<ram>` a new RAM directory):

    gcc -O2 -Wall -o <w>/collorder collorder.c && gcc -O2 -Wall -o <w>/collcontr collcontr.c
    sh <w>/survey_all.sh <w> <ram>/survey 12          # ~8.5 min on a 60-vCPU Zen 1 host under load
    sh <w>/diff_all.sh <w> en_US 10
    LDTIMEOUT=1800 xargs -P 12 -n 1 sh <w>/contr_one.sh <w> <ram>/contr < <w>/locales.txt

The probes are deterministic for a given reference build; there is no spec clause behind them (`no spec clause`):
they are evidence for the derivation, not tests.
