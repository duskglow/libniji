#!/bin/sh
# Black-box probe of the reference loader's LD_PRELOAD and /etc/ld.so.preload behaviour.
# usage: run.sh SCRATCH
set -u
d=$1
rm -rf "$d"; mkdir -p "$d"; cd "$d" || exit 2
lib() { # lib NAME CTOR FVAL [extra]
    printf '#include <unistd.h>\n__attribute__((constructor)) static void c(void) { write(2, "ctor %s\\n", %d); }\nint f(void) { return %s; }\n%s\n' "$1" $((6 + ${#1})) "$3" "${4:-}" > "$1.c"
    cc -O1 -fPIC -shared -o "lib$1.so" "$1.c" -Wl,-soname,"lib$1.so"
}
lib dep 1 2
lib pre 1 1 '#define _GNU_SOURCE
#include <dlfcn.h>
#include <string.h>
size_t strlen(const char *s) { size_t (*n)(const char *) = (size_t (*)(const char *))dlsym(RTLD_NEXT, "strlen"); return n(s) + 1000; }'
lib pre2 1 5
lib epre 1 3
printf '#include <stdio.h>\n#include <string.h>\n#include <unistd.h>\nint f(void);\n__attribute__((constructor)) static void c(void) { write(2, "ctor prog\\n", 10); }\nint main(int c, char **v) { printf("f=%%d strlen=%%zu\\n", f(), strlen(v[0])); return 0; }\n' > prog.c
cc -O1 -o prog prog.c -L. -ldep -Wl,-rpath,"$d"
echo "== ldd --version: $(ldd --version | head -1)"
case_() { echo "== $1"; shift; "$@" 2>&1; echo "rc=$?"; }
case_ "no preload" env -i ./prog
case_ "LD_PRELOAD=./libpre.so" env -i LD_PRELOAD=./libpre.so ./prog
case_ "LD_PRELOAD bare libpre.so (rpath dir of prog)" env -i LD_PRELOAD=libpre.so ./prog
case_ "LD_PRELOAD bare libpre.so with LD_LIBRARY_PATH" env -i LD_LIBRARY_PATH="$d" LD_PRELOAD=libpre.so ./prog
case_ "LD_PRELOAD two, space and colon: pre2 then pre" env -i LD_PRELOAD="$d/libpre2.so:  $d/libpre.so" ./prog
case_ "LD_PRELOAD non-ELF /etc/hostname" env -i LD_PRELOAD=/etc/hostname ./prog
case_ "LD_PRELOAD a directory /tmp" env -i LD_PRELOAD=/tmp ./prog
head -c 100 libpre.so > trunc.so
case_ "LD_PRELOAD truncated ELF (100 bytes)" env -i LD_PRELOAD="$d/trunc.so" ./prog
case_ "LD_PRELOAD missing absolute" env -i LD_PRELOAD=/nonexistent/x.so ./prog
case_ "LD_PRELOAD empty and separators only" env -i LD_PRELOAD=' : ' ./prog
case_ "LD_PRELOAD same lib twice" env -i LD_PRELOAD="$d/libpre.so $d/libpre.so" ./prog
case_ "LD_PRELOAD the dep itself" env -i LD_PRELOAD="$d/libdep.so" ./prog
case_ "LD_PRELOAD with \$ORIGIN" env -i LD_PRELOAD='$ORIGIN/libpre.so' ./prog
command -v strace >/dev/null && { echo "== strace file syscalls, no preload"; strace -f -e trace=%file,openat env -i ./prog 2>&1 | grep -E 'preload|ld.so.cache' ; }
# /etc/ld.so.preload in a user + mount namespace with a private copy of /etc.
mkdir etc
cp -a /etc/. etc/ 2>/dev/null
nscase() { # nscase TITLE CONTENT [ENV...]
    t=$1; c=$2; shift 2
    printf "$c" > etc/ld.so.preload
    echo "== ns: $t"
    unshare -r -m sh -c 'mount --bind "$1/etc" /etc && shift && exec env -i "$@" "$0"' "$d/prog" "$d" "$@" 2>&1
    echo "rc=$?"
}
nscase "file: epre" "$d/libepre.so\n"
nscase "file: epre pre2 (space)" "$d/libepre.so $d/libpre2.so\n"
nscase "file: epre:pre2 (colon)" "$d/libepre.so:$d/libpre2.so\n"
nscase "file: tab and newlines" "$d/libepre.so\t\n\n$d/libpre2.so\n"
nscase "file: comment line" "# $d/libepre.so\n$d/libpre2.so\n"
nscase "file: missing name" "/nonexistent/y.so\n"
nscase "file: bare name" "libepre.so\n"
nscase "file: non-ELF" "/etc/hostname\n"
nscase "file epre + LD_PRELOAD pre2 (which first?)" "$d/libepre.so\n" LD_PRELOAD="$d/libpre2.so"
nscase "file: empty" ""
command -v strace >/dev/null && { printf "$d/libepre.so\n" > etc/ld.so.preload; echo "== ns strace with file"; unshare -r -m sh -c 'mount --bind "$1/etc" /etc && exec strace -e trace=%file,openat,read,close,mmap,munmap,fstat "$0"' "$d/prog" "$d" 2>&1 | grep -B1 -A4 'ld.so.preload' | head -12; }
echo "== done"
