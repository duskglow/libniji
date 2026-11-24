#!/bin/sh
# One locale of the collation survey: build it with the reference localedef in
# a RAM dir, probe its order, record localedef time and LC_COLLATE size, drop it.
# usage: survey_one.sh <workdir> <ramdir> <name e.g. sv_SE or aa_ER@saaho>
w=$1; ram=$2; name=$3
base=${name%%@*}
mod=
case $name in *@*) mod=@${name#*@} ;; esac
dir=$ram/$base.UTF-8$mod
mkdir -p "$ram" "$w/logs" "$w/raw"
s=$(date +%s%N)
timeout ${LDTIMEOUT:-300} localedef -i "$name" -f UTF-8 "$dir" >"$w/logs/$name.localedef" 2>&1
rc=$?
e=$(date +%s%N)
csize=$(stat -c %s "$dir/LC_COLLATE" 2>/dev/null || echo NA)
LOCPATH=$ram timeout 300 "$w/collorder" "$base.UTF-8$mod" 20 D7FF F900 FFFD 10000 1FFFF \
  2>"$w/logs/$name.summary" | gzip -n >"$w/raw/$name.tsv.gz"
prc=$?
printf '%s\tlocaledef_rc=%s\tlocaledef_ms=%s\tLC_COLLATE=%s\tprobe_rc=%s\n' \
  "$name" "$rc" "$(( (e - s) / 1000000 ))" "$csize" "$prc" >"$w/logs/$name.row"
rm -rf "$dir"
