#!/bin/bash
# The interpreter's trace of a trace-at-build `cargo xtask so` (LAYOUT-LDSO-ORDER):
# usage: NIJI_HOT_LISTS=<lists dir> tools/hottext/ldso_trace.sh <tree> --products <dir> --config <path>...
#   <dir> holds this build's interpreter linked ungrouped (ld-linux-x86-64.so.2), its libc.so.6 and
#   niji-crt1.o. Each --config names a configuration file this interpreter opens (its build prefix joined
#   with each of niji_ld::search::CONFIG_FILES, as `cargo xtask so` passes them): every trace sees none.
#   Two dynamic programs of this build, run by that interpreter, are traced with trace_gdb.py
#   (HOT_TEXT_LDSO: the interpreter's functions, first-touch order, every CPU class merged):
#     true   main returns at once: libc.so.6 alone (every dynamic program's start-up);
#     libs3  main calls one function in each of three trivial libraries (the per-object paths).
#   Writes <lists dir>/ldso.<mode>.names; `cargo xtask so` makes ldso-text.list and ldso-text.signatures from
#   them and links the interpreter grouped. Run it only through `cargo xtask so`. libniji binaries only.
#   As regen.sh: the trace is a function of the tree, not of the host or of how fast the trace ran. The
#   `true` trace again with 2 s at every time read (HOT_TEXT_SLOW) and on a simulated one-node host
#   (HOT_TEXT_HOST_NODES=1) must give the same names (TRACE-TIME-PIN, TRACE-NUMA-PIN).
set -eu
export PYTHONDONTWRITEBYTECODE=1
here=$(cd "$(dirname "$0")" && pwd)
T=$(cd "$1" && pwd); shift
L=${NIJI_HOT_LISTS:-}
if [ -z "$L" ] || [ ! -d "$L" ]; then
    echo "ldso_trace: NIJI_HOT_LISTS must name the lists directory of this build (cargo xtask so sets it)" >&2
    exit 1
fi
P=
configs=()
while [ $# -gt 0 ]; do
    case $1 in --products) P=$(cd "$2" && pwd); shift 2;;
        --config) case $2 in /*) configs+=("$2");; *) echo "ldso_trace: --config $2: not an absolute path" >&2; exit 1;; esac; shift 2;;
        *) echo "ldso_trace: unknown argument $1 (usage: NIJI_HOT_LISTS=<dir> ldso_trace.sh <tree> --products <dir> --config <path>...)" >&2; exit 1;; esac
done
[ -n "$P" ] || { echo "ldso_trace: --products <dir> is required" >&2; exit 1; }
[ ${#configs[@]} -gt 0 ] || { echo "ldso_trace: --config <path> is required (the configuration files of the interpreter; cargo xtask so passes them)" >&2; exit 1; }
for f in ld-linux-x86-64.so.2 libc.so.6 niji-crt1.o; do
    [ -f "$P/$f" ] || { echo "ldso_trace: $P/$f missing" >&2; exit 1; }
done
cc=${CC:-cc}
# Under /tmp with a fixed-length name: the programs' interpreter path and run path have the same length on
# every host and in every role (the interpreter reads both).
work=$(mktemp -d /tmp/ldso-trace.XXXXXX)
trap 'rm -rf "$work"' EXIT
cd "$work"
# shellcheck source=tools/hottext/runner.sh
. "$here/runner.sh"
canon_prepare
# The interpreter reads its configuration files (preload list, cache) below its build prefix when they
# exist: every trace sees none (a file of the emptied run directory, never created), whatever the host has.
k=0
for c in "${configs[@]}"; do
    k=$((k + 1))
    hot_redirect="${hot_redirect:+$hot_redirect:}$c=@run/loader-config.$k"
done
# libc.so.6 of <dir> names itself by path (no DT_SONAME): the programs need it at $work/libc.so.6.
cp "$P/ld-linux-x86-64.so.2" "$work/ld.so"
cp "$P/libc.so.6" "$work/libc.so.6"
probe() { # <name> <libraries>: a program of this build calling one function in each library
    local name=$1 n=$2 k libs="" decl="" body=""
    for k in $(seq 1 "$n"); do
        printf 'int f%d(void){return %d;}\n' "$k" "$k" > "l$k.c"
        "$cc" -O1 -fPIC -shared -nostdlib -Wl,-soname,"libl$k.so" "l$k.c" -o "libl$k.so"
        libs="$libs -l:libl$k.so"; decl="$decl int f$k(void);"; body="$body s+=f$k();"
    done
    printf '%s int main(void){long s=0;%s return s==%d?0:1;}\n' "$decl" "$body" $((n * (n + 1) / 2)) > "$name.c"
    # shellcheck disable=SC2086
    "$cc" -O1 -fno-stack-protector -fcf-protection=none -nostdlib -fPIE -pie -Wl,--no-as-needed \
        -Wl,--dynamic-linker="$work/ld.so" "$P/niji-crt1.o" "$name.c" -L. $libs -Wl,-rpath,"$work" \
        "$work/libc.so.6" "$work/ld.so" -lgcc -o "$name"
}
probe true 0
probe libs3 3
run_traced "$work/true.names" "$work/gdb.true.log" HOT_TEXT_LDSO="$work/ld.so" LANG=C -- ./true
run_traced "$work/libs3.names" "$work/gdb.libs3.log" HOT_TEXT_LDSO="$work/ld.so" LANG=C -- ./libs3
run_traced "$work/slow.names" "$work/gdb.slow.log" HOT_TEXT_LDSO="$work/ld.so" HOT_TEXT_SLOW=2 LANG=C -- ./true
run_traced "$work/numa1.names" "$work/gdb.numa1.log" HOT_TEXT_LDSO="$work/ld.so" HOT_TEXT_HOST_NODES=1 LANG=C -- ./true
traces_wait
for m in slow numa1; do
    if ! cmp -s "$work/true.names" "$work/$m.names"; then
        echo "ldso_trace: the interpreter's $m trace differs from its true trace (a host input trace_gdb.py does not answer):" \
             "$(diff "$work/true.names" "$work/$m.names" | grep '^[<>]' | head -20 | tr '\n' ' ')" >&2
        exit 1
    fi
done
for m in true libs3; do
    [ -s "$work/$m.names" ] || { echo "ldso_trace: the $m trace ran no function of the interpreter" >&2; exit 1; }
    cp "$work/$m.names" "$L/ldso.$m.names"
    echo "ldso-text: $m: $(wc -l < "$work/$m.names") names"
done
grep -h "^trace_gdb:" "$work"/gdb.*.log | sort | uniq -c | sed 's/^/ldso-text: /' || true
