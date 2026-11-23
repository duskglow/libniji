#!/bin/bash
# TLS-SURPLUS-PROBE (audit row HL-33): how much TLS objects opened by dlopen after start-up can use,
# on the host C library (the reference) and on libniji, with and without a thread created first,
# under the reference's documented surplus tunables (GLIBC_TUNABLES glibc.rtld.optional_static_tls
# and glibc.rtld.nns, the GNU C Library manual's "Dynamic Linking Tunables").
#
#   bash run.sh <libniji products dir (target/niji)> <new empty work dir>
#
# Black box only: the probe (probe.c) opens objects built from tls_obj.c and reports how many
# opened and the first dlerror text. Paths in the output are written as $W (work dir) and $NIJI.
# No spec clause: a measurement, not a test.
set -u
NIJI=$(cd "$1" && pwd) || exit 2
W=$2
HERE=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$W/obj" || exit 2
CC=${CC:-cc}

clean() { sed -e "s#$W#\$W#g" -e "s#$NIJI#\$NIJI#g"; }

$CC -O2 -o "$W/probe-ref" "$HERE/probe.c" -pthread -ldl || exit 2
# libniji: the same source and the host's headers, run under libniji's loader and C library
# (a program built for the reference, run unchanged).
$CC -O2 -o "$W/probe-niji" "$HERE/probe.c" -pthread -ldl \
    -Wl,--dynamic-linker="$NIJI/ld-linux-x86-64.so.2" -Wl,-rpath,"$NIJI" || exit 2

# obj <model> <size> <copy>: print the path of a TLS object (built once).
obj() {
    local f="$W/obj/$1-$2-$3.so"
    if [ ! -e "$f" ]; then
        $CC -shared -fPIC -O2 -DSIZE="$2" -DMODEL="\"$1\"" "$HERE/tls_obj.c" -o "$f" || { echo "BUILD FAILED $f" >&2; exit 2; }
    fi
    echo "$f"
}

# run <libc ref|niji> <threads> <tunables|-> <object>...: one probe process; prints its line.
run() {
    local libc=$1 thr=$2 tun=$3
    shift 3
    local envs=(env -u GLIBC_TUNABLES)
    [ "$tun" != "-" ] && envs+=(GLIBC_TUNABLES="$tun")
    if [ "$libc" = niji ]; then
        "${envs[@]}" LD_LIBRARY_PATH="$NIJI" timeout 30 "$W/probe-niji" "$thr" "$@" 2>&1
    else
        "${envs[@]}" timeout 30 "$W/probe-ref" "$thr" "$@" 2>&1
    fi
}

# bisect <libc> <threads> <tunables> <model> [prefix objects]: the largest single block of <model>
# (16-byte steps) that opens after the prefix objects.
bisect() {
    local libc=$1 thr=$2 tun=$3 model=$4
    shift 4
    local lo=16 hi=4194304 mid out last_fail=""
    out=$(run "$libc" "$thr" "$tun" "$@" "$(obj "$model" $lo 0)")
    case "$out" in *" ok") ;; *) echo "largest=<16 ($out)" | clean; return;; esac
    out=$(run "$libc" "$thr" "$tun" "$@" "$(obj "$model" $hi 0)")
    case "$out" in *" ok") echo "largest>=$hi" ; return;; esac
    last_fail=$out
    while [ $((hi - lo)) -gt 16 ]; do
        mid=$(( (lo + hi) / 2 / 16 * 16 ))
        out=$(run "$libc" "$thr" "$tun" "$@" "$(obj "$model" $mid 0)")
        case "$out" in
            *" ok") lo=$mid ;;
            *"fail:"*) hi=$mid; last_fail=$out ;;
            *) echo "BAD at $mid: $out" | clean; return ;;
        esac
    done
    echo "largest=$lo fails_at=$hi [$last_fail]" | clean
}

# cumulative <libc> <threads> <tunables> <model> <size> <copies>: how many separate objects of
# <size> bytes open one after another.
cumulative() {
    local libc=$1 thr=$2 tun=$3 model=$4 size=$5 n=$6 i list=()
    for ((i = 0; i < n; i++)); do list+=("$(obj "$model" "$size" "$i")"); done
    run "$libc" "$thr" "$tun" "${list[@]}" | clean
}

echo "# host C library: $(ldd --version | head -1)"
f=$(obj initial-exec 1024 0)
echo "# initial-exec object: $(readelf -lW "$f" | grep -m1 ' TLS ' | tr -s ' ') / $(readelf -dW "$f" | grep -m1 FLAGS | tr -s ' ')"
f=$(obj global-dynamic 1024 0)
echo "# global-dynamic object: $(readelf -lW "$f" | grep -m1 ' TLS ' | tr -s ' ') / flags: $(readelf -dW "$f" | grep -c STATIC_TLS) STATIC_TLS"

TUNS=(- glibc.rtld.optional_static_tls=0 glibc.rtld.optional_static_tls=512 glibc.rtld.optional_static_tls=4096
      glibc.rtld.optional_static_tls=65536 glibc.rtld.optional_static_tls=1048576 glibc.rtld.nns=1 glibc.rtld.nns=16)

echo
echo "## A. largest single initial-exec block opened by dlopen (bytes, 16-byte steps)"
for libc in ref niji; do for thr in 0 1; do for tun in "${TUNS[@]}"; do
    echo "A libc=$libc threads=$thr tunables=$tun: $(bisect $libc $thr "$tun" initial-exec)"
done; done; done

echo
echo "## B. 1 KiB initial-exec objects opened one after another (of 160)"
for libc in ref niji; do for thr in 0 1; do for tun in "${TUNS[@]}"; do
    echo "B libc=$libc threads=$thr tunables=$tun: $(cumulative $libc $thr "$tun" initial-exec 1024 160)"
done; done; done

echo
echo "## C. 16-byte initial-exec objects opened one after another (of 1100; default tunables)"
for libc in ref niji; do for thr in 0 1; do
    echo "C libc=$libc threads=$thr: $(cumulative $libc $thr - initial-exec 16 1100)"
done; done

echo
echo "## D. 16 KiB global-dynamic objects opened one after another (of 64 = 1 MiB; default tunables)"
for libc in ref niji; do for thr in 0 1; do
    echo "D libc=$libc threads=$thr: $(cumulative $libc $thr - global-dynamic 16384 64)"
done; done

echo
echo "## E. largest single initial-exec block after four 16 KiB global-dynamic objects (default tunables)"
gd=()
for i in 0 1 2 3; do gd+=("$(obj global-dynamic 16384 "$i")"); done
for libc in ref niji; do for thr in 0 1; do
    echo "E libc=$libc threads=$thr: $(bisect $libc $thr - initial-exec "${gd[@]}")"
done; done
echo "# done"
