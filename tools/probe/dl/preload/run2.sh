#!/bin/sh
# Second probe: /etc/ld.so.preload comment and separator details. usage: run2.sh PROBE1DIR
set -u
d=$1
cd "$d" || exit 2
nscase() { # nscase TITLE CONTENT
    printf "$2" > etc/ld.so.preload
    echo "== ns: $1"
    unshare -r -m sh -c 'mount --bind "$1/etc" /etc && exec "$0"' "$d/prog" "$d" 2>&1
    echo "rc=$?"
}
nscase "hash glued after a name: epre#pre2" "$d/libepre.so#$d/libpre2.so\n"
nscase "hash after space: epre #pre2 then pre on next line" "$d/libepre.so #$d/libpre2.so\n$d/libpre.so\n"
nscase "CR before newline" "$d/libepre.so\r\n"
nscase "comma" "$d/libepre.so,$d/libpre2.so\n"
nscase "no trailing newline" "$d/libpre2.so"
nscase "LD_PRELOAD-style tab inside env (not file): see below" ""
echo "== env tab separator"; env -i LD_PRELOAD="$d/libpre2.so	$d/libpre.so" ./prog 2>&1; echo "rc=$?"
echo "== env newline separator"; env -i LD_PRELOAD="$d/libpre2.so
$d/libpre.so" ./prog 2>&1; echo "rc=$?"
echo "== done"
