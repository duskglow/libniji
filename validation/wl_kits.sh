#!/bin/bash
# LAB-ONLY: this script drives the project's own test hosts and is not needed to build or test libniji.
# Static vs dynamic libniji vs glibc on the workloads (WL-DYNAMIC-KIT): one table, one head, one host.
# THE STATIC SWEEP (WL-DYNAMIC-GATE, owner ruling #6965, D-X.141): since the per-gate counts bind the dynamic kit, this
# runs nightly on a build host and in every release candidate's checklist, and prints the INVIOLABLE rule's verdict for
# both kits (a static FAIL opens a story; it does not block a gate).
#
#   validation/wl_kits.sh SHA [WORKLOAD...]        (default: every workload of wl_workloads.sh)
#
# RUNS ON THE HOST WHERE IT IS STARTED, under jobrun:
#   ~/bin/jobrun <label> count -- sh -c 'cd ~/w/<role> && bash validation/wl_kits.sh <full sha>'
# SHA's source must be in ~/wl/src/SHA (wl_count.sh or wl_prime_base.sh put it there). Steps, each timed:
#   1. wl_prep.sh twice: the static kit (~/wl/kit/SHA-release, libc.a; the nightly sweep's) and the dynamic kit
#      (WL_LINK=dynamic, ~/wl/kit/SHA-release-dyn: libniji's libc.so.6 and ld.so, wl_dyncc.sh), only the programs
#      the workloads use; the glibc kit is the host build, dynamic.
#   2. per workload, WLK_RUNS (default 5) rounds, glibc / static / dynamic interleaved in each round: perf stat
#      instructions:u, instructions:k, page-faults, task-clock, and the wall time; one output md5 per column; one
#      `strace -f -e trace=execve` (static column): how many times each kit program starts.
#   3. per program, 3 runs per column of `<prog> --version`: the instructions and faults of one start (loader,
#      relocation, IFUNC, libc and program initialisation, exit).
#   4. wl_kits.py: medians, ratios to glibc, geomeans, the workloads above 1.00 and 1.10 per column, and the part of
#      each dynamic-minus-static difference that is start-up (execs x per-start difference).
# Time (task-clock, wall) is ADVISORY: a build host, not a booked timing host; instructions and faults are the
# deterministic columns. Raw rows and the table: ~/wl/kits/SHA-release/.
set -u
here=$(cd "$(dirname "$0")" && pwd)
WL=${WL:-$HOME/wl}
[ $# -ge 1 ] || { sed -n '2,/^set -u/p' "$0" | sed '$d'; exit 2; }
sha=$1
shift
. "$here/wl_workloads.sh"
. "$here/wl_lock.sh"
runs=${WLK_RUNS:-5}
want=("$@")
if [ ${#want[@]} = 0 ]; then for d in "${WL_DEFS[@]}"; do want+=("${d%%|*}"); done; fi
progs=
for w in "${want[@]}"; do
    hit=0
    for d in "${WL_DEFS[@]}"; do
        [ "${d%%|*}" = "$w" ] || continue
        hit=1
        for p in $(echo "${d#*|}" | grep -o '@/[a-z0-9]*' | cut -c3-); do
            case " $progs " in *" $p "*) ;; *) progs="$progs $p" ;; esac
        done
    done
    [ $hit = 1 ] || { echo "wl_kits: unknown workload $w" >&2; exit 2; }
done
progs=${progs# }
[ -f "$WL/src/$sha/Cargo.toml" ] || { echo "wl_kits: no source in $WL/src/$sha (run wl_count.sh on it first, or wl_prime_base.sh)" >&2; exit 2; }

t0=$(date +%s)
for l in static dynamic; do
    s=$(date +%s)
    WL_LINK=$l WL_PROGS="$progs" bash "$here/wl_prep.sh" "$sha" || { echo "wl_kits: wl_prep ($l) failed" >&2; exit 1; }
    echo "wl_kits: step prep-$l $(( $(date +%s) - s )) s"
done
declare -A kit
kit[glibc]=$WL/kit/glibc/bin
kit[static]=$WL/kit/$(WL_LINK=static wl_key "$sha")/bin
kit[dynamic]=$WL/kit/$(WL_LINK=dynamic wl_key "$sha")/bin
cols="glibc static dynamic"
out=$WL/kits/$(WL_LINK=static wl_key "$sha")
mkdir -p "$out" || exit 1
raw=$out/raw.tsv
: > "$raw.$$"
work=$WL/work
tmp=$(mktemp -d "$WL/tmp.kits.XXXXXX") || exit 1
trap 'rm -rf "$tmp" "$raw.$$"' EXIT
[ -d "$work/tree" ] || { mkdir -p "$work/tree" && tar -xf "$work/tree.tar" -C "$work/tree"; }
ev() { grep ",$2," "$1" | tail -1 | cut -d, -f1; }

s=$(date +%s)
for w in "${want[@]}"; do
    cmd=
    for d in "${WL_DEFS[@]}"; do [ "${d%%|*}" = "$w" ] && cmd=${d#*|}; done
    for col in $cols; do   # warm-up (page cache), not counted
        (cd "$work" && sh -c "${cmd//@/${kit[$col]}}" 2>/dev/null | cat > /dev/null)
    done
    for ((r = 1; r <= runs; r++)); do
        for col in $cols; do
            c=${cmd//@/${kit[$col]}}
            a=$(date +%s%N)
            (cd "$work" && perf stat -x, -o "$tmp/perf.txt" -e instructions:u,instructions:k,page-faults,task-clock sh -c "$c" 2>/dev/null | cat > /dev/null)
            b=$(date +%s%N)
            printf 'run\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$w" "$col" "$r" "$(ev "$tmp/perf.txt" instructions:u)" \
                "$(ev "$tmp/perf.txt" instructions:k)" "$(ev "$tmp/perf.txt" page-faults)" "$(ev "$tmp/perf.txt" task-clock)" \
                "$(( (b - a) / 1000 ))" >> "$raw.$$"
        done
    done
    ref=
    for col in $cols; do
        sum=$(cd "$work" && sh -c "${cmd//@/${kit[$col]}}" 2>/dev/null | md5sum | cut -c1-12)
        printf 'md5\t%s\t%s\t%s\n' "$w" "$col" "$sum" >> "$raw.$$"
        [ "$col" = glibc ] && ref=$sum
        # A differing output: wl_run.sh's fingerprint of the differing lines (count:sha1), which wl_kits.py checks
        # against wl_known_diffs.tsv (the workload gate's list of accepted differences).
        if [ "$col" != glibc ] && [ "$sum" != "$ref" ]; then
            (cd "$work" && sh -c "${cmd//@/${kit[glibc]}}" > "$tmp/ref.out" 2>/dev/null; sh -c "${cmd//@/${kit[$col]}}" > "$tmp/new.out" 2>/dev/null)
            printf 'dfp\t%s\t%s\t%s\n' "$w" "$col" "$(diff -a "$tmp/ref.out" "$tmp/new.out" | grep -a -c '^[<>]'):$(diff -a "$tmp/ref.out" "$tmp/new.out" | grep -a '^[<>]' | sha1sum | cut -c1-12)" >> "$raw.$$"
            rm -f "$tmp/ref.out" "$tmp/new.out"
        fi
    done
    (cd "$work" && strace -f -e trace=execve -o "$tmp/exec.txt" sh -c "${cmd//@/${kit[static]}}" 2>/dev/null | cat > /dev/null)
    for p in $progs; do
        n=$(grep -c "execve(\"${kit[static]}/$p\"" "$tmp/exec.txt")
        [ "$n" = 0 ] || printf 'exec\t%s\t%s\t%s\n' "$w" "$p" "$n" >> "$raw.$$"
    done
done
echo "wl_kits: step workloads $(( $(date +%s) - s )) s ($runs rounds)"

for p in $progs; do
    for ((r = 1; r <= 3; r++)); do
        for col in $cols; do
            perf stat -x, -o "$tmp/perf.txt" -e instructions:u,instructions:k,page-faults "${kit[$col]}/$p" --version > /dev/null 2>&1
            printf 'start\t%s\t%s\t%s\t%s\t%s\t%s\n' "$p" "$col" "$r" "$(ev "$tmp/perf.txt" instructions:u)" \
                "$(ev "$tmp/perf.txt" instructions:k)" "$(ev "$tmp/perf.txt" page-faults)" >> "$raw.$$"
        done
    done
done
mv -f "$raw.$$" "$raw"
python3 "$here/wl_kits.py" "$raw" | tee "$out/table.txt"
echo "wl_kits: exit 0, $(( $(date +%s) - t0 )) s wall (raw $raw, table $out/table.txt)"
