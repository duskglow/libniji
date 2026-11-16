#!/bin/sh
# tools/deb/swap-cmds.sh OUTDIR: the fixed command set tools/deb/swap-system.sh runs inside a Debian root,
# once on the distribution's C library and once with Liella's system packages swapped in. No spec clause:
# release validation (plan/system-libc.md section 3.7). Every command is one of the distribution's own
# programs on a small fixed input; each writes OUTDIR/<id>.out (stdout and stderr) and OUTDIR/<id>.rc.
# OUTDIR/info.txt holds what is expected to differ between the roots (the C library's identity).
# The input's times are fixed and listings are sorted (no dependence on directory order).
set -u
out=$1
mkdir -p "$out"
export LC_ALL=C TZ=UTC PATH=/usr/sbin:/usr/bin:/sbin:/bin HOME=/root
unset LANG LANGUAGE
t=/tmp/t
rm -rf "$t"
mkdir -p "$t/sub/deeper" "$t/empty"
printf 'banana 3 yellow\napple 10 red\ncherry 7 dark red\nApple 2 green\ndate 15 brown\n\303\251lan 1 accent\napple 10 red\n' > "$t/in.txt"
printf 'one\ntwo\n' > "$t/sub/a.txt"
printf 'x' > "$t/sub/deeper/b.dat"
: > "$t/sub/zero.txt"
chmod 0640 "$t/sub/a.txt"
find "$t" -exec touch -h -d @86400 {} +
touch -d @172800 "$t/sub/deeper/b.dat"

# run ID CMD...: run CMD (60 s hang guard) and keep its output and status.
run() {
    id=$1; shift
    timeout 60 "$@" > "$out/$id.out" 2>&1 < /dev/null
    echo $? > "$out/$id.rc"
}
# sh_ ID SCRIPT: run SCRIPT with sh -c (for pipes into sort).
sh_() { run "$1" sh -c "$2"; }

run ls-1 ls "$t"
run ls-2 ls -l --time-style=+%s "$t" "$t/sub"
sh_ ls-3 "ls -aR $t"
run ls-4 ls -1 --sort=size -r "$t/sub"
run ls-5 ls /nonexistent
run sort-1 sort "$t/in.txt"
run sort-2 sort -n -k2 "$t/in.txt"
run sort-3 sort -r -t ' ' -k3,3 -k1,1 "$t/in.txt"
run sort-4 sort -f -u "$t/in.txt"
run sort-5 sort -c "$t/in.txt"
run grep-1 grep -n an "$t/in.txt"
run grep-2 grep -c -i apple "$t/in.txt"
run grep-3 grep -E '^[a-c].* [0-9]{2} ' "$t/in.txt"
run grep-4 grep -o -w red "$t/in.txt"
run grep-5 grep -v -F e "$t/in.txt"
sh_ grep-6 "grep -r --include='*.txt' -l o $t | sort"
run grep-7 grep zzz "$t/in.txt"
run sed-1 sed 's/\([a-z]*\) \([0-9]*\)/\2:\1/' "$t/in.txt"
run sed-2 sed -n '/red/{=;p}' "$t/in.txt"
run sed-3 sed -E 's/[aeiou]+/<&>/g;y/abc/ABC/' "$t/in.txt"
run sed-4 sed -e '1!G;h;$!d' "$t/in.txt"
run sed-5 sed 's/unterminated' "$t/in.txt"
run awk-1 awk '{s+=$2} END {print s, NR, s/NR}' "$t/in.txt"
run awk-2 awk '{printf "%-8s|%6.3f|%e|%5d\n", $1, sqrt($2), exp($2), $2*1000}' "$t/in.txt"
run awk-3 awk '$2>5 {print toupper($1), length($0), substr($0, 3, 4)}' "$t/in.txt"
run awk-4 awk 'BEGIN {print sprintf("%.10g", 1/3), int(-3.7), 2^0.5, sin(1), log(10), atan2(0, -1); x["b"]=1; x["a"]=2; n=split("q,w,e", y, ","); print n, y[3]}'
run awk-5 awk -F ' ' '{c[$3]++} END {for (k in c) print k, c[k] | "sort"}' "$t/in.txt"
run bash-1 bash -c 'a=(3 1 2); for i in "${a[@]}"; do echo $((i*i)); done; printf "%s %5.2f %x %q\n" x 3.14159 255 "a b"; [[ abc == a* ]] && echo match; echo ${#a[@]} $((2**40)); x=$(echo sub); echo "$x"; f() { return 3; }; f || echo rc=$?'
run bash-2 bash -c 'read -r a b c < /tmp/t/in.txt; echo "$c-$b-$a"; while IFS=" " read -r w n rest; do echo "${w^^}:$n"; done < /tmp/t/in.txt | head -3; echo ${BASH_VERSINFO[0]} >/dev/null; shopt -s nullglob; echo /tmp/t/sub/*.txt'
run bash-3 bash -c 'echo out; echo err >&2; exit 7'
run bash-4 bash -c 'trap "echo trapped" USR1; kill -USR1 $$; (sleep 0; exit 5) & wait $!; echo child=$?; set -o pipefail; false | true; echo pipe=$?'
run perl-1 perl -e 'print join(",", sort map { $_ * $_ } 1 .. 5), "\n"; printf("%.15g %s\n", atan2(1, 1) * 4, join(",", sort { $a <=> $b } (10, 9, 100, 1))); my %h = (b => 2, a => 1); print join(",", map {"$_=$h{$_}"} sort keys %h), "\n"; print scalar(gmtime(86400 * 365)), "\n"'
run perl-2 perl -ne 'print "$.: $_" if /an/' "$t/in.txt"
run perl-3 perl -e 'use POSIX qw(strftime floor ceil fmod); print strftime("%Y-%m-%d %H:%M:%S %j", gmtime(1e9)), " ", floor(-2.5), " ", ceil(2.1), " ", fmod(7, 3), "\n"; printf "%e %g %.3f\n", 1e300 * 10, 0.0001234, -0.0005; my @s = split /,/, "a,b,,c"; print scalar(@s), "\n"'
run perl-4 perl -e 'open(my $f, "<", "/tmp/t/in.txt") or die; my @l = <$f>; print scalar(@l), " ", length(join("", @l)), "\n"; my $s = sprintf("%vd", "1.22.333"); print "$s\n"; die "boom\n"'
run perl-5 perl -e 'my $pid = fork(); if ($pid == 0) { exit 3 } waitpid($pid, 0); print $? >> 8, "\n"; print join(" ", (getpwuid(0))[0, 2, 7]), "\n"'
run python3-1 python3 -c 'import math, json, struct; print(math.sqrt(2), math.lgamma(5.5), math.fsum([0.1] * 10), json.dumps({"b": [1, 2.5, None], "a": "é"}, sort_keys=True)); print(struct.pack("<id", 7, 1.5).hex()); print(sorted(["b", "a", "C"]), "%.17g" % 0.1, float("1e308") * 10, 10 ** 30 // 7)'
run python3-2 python3 -c 'import os, sys, time, hashlib; print(sys.getfilesystemencoding(), sys.stdout.encoding, os.getcwd()); print(time.strftime("%Y %j %a %b", time.gmtime(10 ** 9))); print(hashlib.sha256(b"liella").hexdigest()); print(sorted(os.listdir("/tmp/t/sub")))'
run python3-3 python3 -c 'import subprocess; r = subprocess.run(["sort", "-r", "/tmp/t/in.txt"], capture_output=True, text=True); print(r.returncode, r.stdout, end="")'
run python3-4 python3 -c 'import threading; out = []; ts = [threading.Thread(target=lambda i=i: out.append(i * i)) for i in range(8)]; [t.start() for t in ts]; [t.join() for t in ts]; print(sorted(out))'
run python3-5 python3 -c 'import pwd, locale; print(pwd.getpwuid(0).pw_name, locale.setlocale(locale.LC_ALL, "")); raise SystemExit(4)'
run python3-6 python3 -c '1/0'
# ldd (UTIL-4) on the distribution's own libc-linked programs and libraries. A load address changes from
# run to run in both roots: only a line's final " (0x<hex>)" is masked, the rest of the line is compared.
ldd_() { # ldd_ ID ARG...: ldd ARG... with the addresses masked; ldd's own status is kept.
    id=$1; shift
    timeout 60 ldd "$@" > "$out/$id.raw" 2>&1 < /dev/null
    echo $? > "$out/$id.rc"
    sed 's/ (0x[0-9a-f]*)$/ (@ADDR@)/' "$out/$id.raw" > "$out/$id.out"
    rm -f "$out/$id.raw"
}
py=$(readlink -f /usr/bin/python3)
ldd_ ldd-1 /bin/sh
ldd_ ldd-2 /usr/bin/perl
ldd_ ldd-3 "$py"
ldd_ ldd-4 /usr/lib/x86_64-linux-gnu/libz.so.1
ldd_ ldd-5 /bin/sh /usr/bin/perl
ldd_ ldd-6 -v /bin/sh
ldd_ ldd-7 -v "$py"
ldd_ ldd-8 /etc/passwd
# initramfs-tools' copy_exec, which builds an initramfs from ldd's listing: /bin/sh, perl and python3 with
# their libraries into an empty directory; the copied tree (type and path, sorted) is compared.
sh_ copyexec-1 "verbose=n && . /usr/share/initramfs-tools/hook-functions && DESTDIR=/tmp/ird && rm -rf \$DESTDIR && mkdir -p \$DESTDIR && copy_exec /bin/sh /bin && copy_exec /usr/bin/perl /bin && copy_exec $py /bin && cd \$DESTDIR && find . -printf '%y %p\n' | sort"
sh_ find-1 "find $t | sort"
sh_ find-2 "find $t -type f -name '*.txt' -size -2k -printf '%P %s %m %TY\n' | sort"
sh_ find-3 "find $t -newer $t/in.txt | sort; find $t -empty | sort"
sh_ find-4 "find $t -maxdepth 1 -type d -exec echo dir {} \\; | sort"
run find-5 find /nonexistent
# fakeroot (DL-LD-PRELOAD): it preloads its library (LD_PRELOAD) into the programs it starts, and that
# library reaches libc's functions with dlsym(RTLD_NEXT): the fake root shows only when both work. Run
# as an unprivileged user (65534, perl drops root first): inside, id and a chowned file show 0;
# outside, the user and the file stay 65534. A tar archive written under fakeroot holds 0/0, the
# same tree archived outside 65534/65534 (--numeric-owner; name, owner and type columns only).
mkdir -p /tmp/fr
chmod 1777 /tmp/fr
drop='$) = "65534 65534"; $( = 65534; $< = $> = 65534; exec @ARGV or die "exec: $!\n"'
run fakeroot-1 perl -e "$drop" -- sh -c 'cd /tmp/fr && fakeroot sh -c "touch f && chown 0:0 f && echo inside \$(id -u) \$(id -g) \$(stat -c %u:%g f)" && echo outside $(id -u) $(id -g) $(stat -c %u:%g f)'
run fakeroot-2 perl -e "$drop" -- sh -c 'cd /tmp/fr && mkdir d && echo x > d/a && fakeroot sh -c "chown -R 0:0 d && tar --numeric-owner -cf a.tar d" && tar --numeric-owner -tvf a.tar | awk "{print substr(\$1, 1, 1), \$2, \$6}" && tar --numeric-owner -cf b.tar d && tar --numeric-owner -tvf b.tar | awk "{print substr(\$1, 1, 1), \$2, \$6}"'

{
    echo "libc6 $(dpkg-query -W -f '${Version}' libc6 2>&1)"
    echo "libc-bin $(dpkg-query -W -f '${Version}' libc-bin 2>&1)"
    sha256sum /usr/lib/x86_64-linux-gnu/libc.so.6 /usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2 2>&1
    echo "lib64 loader -> $(readlink -f /lib64/ld-linux-x86-64.so.2)"
    echo "ldconfig -p: $(ldconfig -p 2>&1 | grep -c 'libc\.so\.6') libc.so.6 entries, $(ldconfig -p 2>&1 | head -1)"
} > "$out/info.txt"
echo "swap-cmds: done $(ls "$out" | grep -c '\.rc$') commands"
