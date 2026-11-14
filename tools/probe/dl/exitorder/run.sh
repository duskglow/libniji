#!/bin/sh
# run.sh - black-box probe of the exit-time destructor order of dlopen()ed objects still loaded at
# exit (story DL-EXIT-ORDER-PROBE; probe.c gives the OP letters). Builds in SCRATCH (created, must not
# exist) and prints the record; each scenario is its own process. Public interfaces only; the
# reference's loader and libraries are only executed.
#   sh tools/probe/dl/exitorder/run.sh SCRATCH
# With PROBE_RUN set (for example "/path/ld.so --library-path DIR"), each probe runs under it.
set -u
SRC=$(cd "$(dirname "$0")" && pwd)
W=${1:?usage: run.sh SCRATCH}
[ -e "$W" ] && { echo "run.sh: $W exists" >&2; exit 2; }
mkdir -p "$W" || exit 2
W=$(cd "$W" && pwd)
CC=${CC:-cc}
export LC_ALL=C
unset LD_LIBRARY_PATH LD_PRELOAD LD_BIND_NOW GLIBC_TUNABLES 2>/dev/null
lib() { n=$1; shift; $CC -std=gnu17 -O1 -fPIC -shared -DNAME="$n" -L"$W" -Wl,-rpath,"$W" -Wl,--no-as-needed "$@" -o "$W/lib$n.so" "$SRC/lib.c" || exit 2; }
for n in A B C N P L; do lib $n; done
for n in Q R S; do lib $n -DUNIQ; done
lib D -l:libC.so     # D needs C
lib H2 -l:libD.so    # H2 needs D (needs C)
lib M -l:libL.so     # M needs L, a start-time object of probeL
lib I                # the cycle H <-> I: I first without H, then H, then I again with H
lib H -l:libI.so
lib I -l:libH.so
lib Z                # the cycle X -> Y -> Z -> X
lib Y -l:libZ.so
lib X -l:libY.so
lib Z -l:libX.so
lib K -l:libY.so     # K needs Y, a member of that cycle
lib GD -DGDEF
lib GU -DGUSE
$CC -std=gnu17 -O1 -o "$W/probe" "$SRC/probe.c" ${PROBE_LDFLAGS:-} || exit 2
# probeL: the same program with libL.so as a start-time object (DT_NEEDED).
$CC -std=gnu17 -O1 -o "$W/probeL" "$SRC/probe.c" -L"$W" -Wl,-rpath,"$W" -Wl,--no-as-needed -l:libL.so ${PROBE_LDFLAGS:-} || exit 2
BIN=probe
run() { # run TAG OP...
    tag=$1; shift
    echo "== $tag ($BIN): $*"
    # shellcheck disable=SC2086
    timeout 60 ${PROBE_RUN:-} "$W/$BIN" "$W" "$@" 2>&1 | sed "s|$W|<dir>|g"
    echo "rc=$?"
}
run E1-open3 A:k B:k C:k
run E2-nodelete3-closed A:n B:n C:n
run E3-nodelete3-open A:N B:N C:N
run E4-unique3 Q:u R:u S:u
run E5-dlclose-c6-c7 P:x Q:x Q:u N:n
run E6-unique-then-nodelete Q:u A:n
run E7-nodelete-then-unique A:n Q:u
run E8-open-nodelete-open A:k B:n C:k
run E9-nodelete-open-nodelete A:n B:k C:n
run E10-unique-then-open Q:u A:k
run E11-open-then-unique A:k Q:u
run E12-unique-unbound-open Q:k R:k
run E13-reopen-first A:k B:k A:k
run E14-dependency-loaded-first C:k D:k
run E15-plain-then-dependent A:k D:k
run E16-dependent-then-plain D:k A:k
run E17-dependency-plain-dependent C:k A:k D:k
run E18-chain-reorder D:k H2:k
run E19-cycle H:k
run E20-cycle-between A:k H:k B:k
run E21-global-binding GD:g GU:k
run E22-nodelete-dependency C:n D:n
run E26-cycle3-from-X X:k
run E27-cycle3-from-Y Y:k
run E28-cycle3-between A:k X:k B:k
run E29-cycle3-then-dependent X:k K:k
run E30-dependent-then-cycle3 K:k
BIN=probeL
run E23-needs-start-time-object M:k
run E24-start-time-object-then-plain A:k
run E25-start-time-mixed A:k M:k B:k
