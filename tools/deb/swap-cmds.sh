#!/bin/sh
# tools/deb/swap-cmds.sh OUTDIR: the fixed command set tools/deb/swap-system.sh runs inside a Debian root,
# once on the distribution's C library and once with Liella's system packages swapped in. No spec clause:
# release validation (plan/system-libc.md section 3.7). Every command is one of the distribution's own
# programs on a small fixed input (and apt, a C++ program, on the root's own package lists); each writes OUTDIR/<id>.out (stdout and stderr) and OUTDIR/<id>.rc.
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
sh_ find-1 "find $t | sort"
sh_ find-2 "find $t -type f -name '*.txt' -size -2k -printf '%P %s %m %TY\n' | sort"
sh_ find-3 "find $t -newer $t/in.txt | sort; find $t -empty | sort"
sh_ find-4 "find $t -maxdepth 1 -type d -exec echo dir {} \\; | sort"
run find-5 find /nonexistent
# apt is C++: it starts with the distribution's shared libstdc++.so.6 (unique symbols, DL-LIBSTDCXX).
# apt-get check (apt-1) reads the installed packages' dependencies. In the swapped root it reports the
# ones Liella's libc6 cannot meet by version (2.39 against a newer suite's "libc6 (>= 2.41)"; the swap
# installs it with --force-depends), with status 100. Those lines, and only those (the installed version
# must be Liella's), are dropped in both roots, and status 100 counts as 0 when they were apt's only
# complaint. Anything else apt reports (another unmet dependency, a loader error, a crash's status)
# still differs from the stock root. The unfiltered output stays in OUTDIR/apt-1.raw.
apt_check='apt-get check > "$1" 2>&1; rc=$?
kept=$(grep -v -e "^You might want to run .apt --fix-broken install" -e "^The following packages have unmet dependencies:" \
    -e "Depends: libc6 (>= [^)]*) but [^ ]*+liella[^ ]* is installed" -e "^E: Unmet dependencies\. " "$1")
printf "%s\n" "$kept"
if [ "$rc" -eq 100 ] && grep -q "+liella[^ ]* is installed" "$1" && ! printf "%s\n" "$kept" | grep -q -e "Depends:" -e "^[EW]: "; then
    exit 0
fi
exit "$rc"'
run apt-1 sh -c "$apt_check" sh "$out/apt-1.raw"
run apt-2 apt-cache policy
run apt-3 apt-cache policy bash

{
    echo "libc6 $(dpkg-query -W -f '${Version}' libc6 2>&1)"
    echo "libc-bin $(dpkg-query -W -f '${Version}' libc-bin 2>&1)"
    sha256sum /usr/lib/x86_64-linux-gnu/libc.so.6 /usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2 2>&1
    echo "lib64 loader -> $(readlink -f /lib64/ld-linux-x86-64.so.2)"
    echo "ldconfig -p: $(ldconfig -p 2>&1 | grep -c 'libc\.so\.6') libc.so.6 entries, $(ldconfig -p 2>&1 | head -1)"
} > "$out/info.txt"
echo "swap-cmds: done $(ls "$out" | grep -c '\.rc$') commands"
