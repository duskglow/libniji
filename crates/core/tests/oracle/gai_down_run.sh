#!/bin/sh
# Run gai_down.c (spec §16.1-16.4; GitHub issue #8): write every scenario's fixture /etc, then run
# each scenario in private user, mount and network namespaces (`unshare -r -m -n`, no root needed)
# where only the loopback interface is up, with the fixture bind-mounted over /etc and a fresh
# tmpfs over /dev/shm (libniji's shared name-service cache, D-16.10, starts empty and leaves
# nothing behind). The program is copied into the fixture and run from there, so a tree under
# /dev/shm (a `cargo xtask plants` copy in RAM) does not vanish under the new tmpfs. The dynamic
# loader's cache is copied into each fixture so that a dynamically linked program still starts.
#
# Usage: gai_down_run.sh PROGRAM SCRATCH
set -eu
prog=$1
dir=$2
rm -rf "$dir"
mkdir -p "$dir"
for s in $("$prog" "$dir"); do
    if [ -f /etc/ld.so.cache ]; then
        cp /etc/ld.so.cache "$dir/$s/etc/ld.so.cache"
    fi
    cp "$prog" "$dir/$s/etc/.probe"
    unshare -r -m -n sh -c 'ip link set lo up && mount --bind "$1/etc" /etc && mount -t tmpfs tmpfs /dev/shm && exec /etc/.probe /etc "$2"' sh "$dir/$s" "$s"
done
