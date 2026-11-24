#!/bin/sh
# Contraction pass for one locale: build it with the reference localedef in a
# RAM dir, run collcontr over the Latin candidate set plus the code points the
# locale moves against the root (when at most 300), drop the build.
# usage: contr_one.sh <workdir> <ramdir> <name>
w=$1; ram=$2; name=$3
base=${name%%@*}
mod=
case $name in *@*) mod=@${name#*@} ;; esac
dir=$ram/$base.UTF-8$mod
mkdir -p "$ram" "$w/contr"
timeout "${LDTIMEOUT:-300}" localedef -i "$name" -f UTF-8 "$dir" >/dev/null 2>&1
latin=$(python3 -c '
r=list(range(0x41,0x5b))+list(range(0x61,0x7b))+[c for c in range(0xc0,0x100) if c not in (0xd7,0xf7)]+list(range(0x100,0x180))
print(" ".join("%X"%c for c in r))')
moved=$(grep '^&' "$w/rules/$name.rules" 2>/dev/null | awk '{print substr($3,3)}' | sort -u)
nm=$(printf '%s\n' "$moved" | grep -c .)
[ "$nm" -le 300 ] || moved=
cand=$(printf '%s\n' $latin $moved | sort -u)
LOCPATH=$ram timeout 600 "$w/collcontr" "$base.UTF-8$mod" $cand >"$w/contr/$name.contr" 2>"$w/contr/$name.err"
printf '%s\tmoved=%s\tcandidates=%s\trc=%s\n' "$name" "$nm" "$(printf '%s\n' $cand | grep -c .)" "$?" >>"$w/contr/$name.err"
rm -rf "$dir"
