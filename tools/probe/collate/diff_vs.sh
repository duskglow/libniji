#!/bin/sh
# Diff the named surveyed locales against another base locale (summary lines only).
# usage: diff_vs.sh <workdir> <base> <locale>...
w=$1; base=$2; shift 2
gzip -dc "$w/raw/$base.tsv.gz" >"$w/tmp/base-$base.tsv"
for l in "$@"; do
  gzip -dc "$w/raw/$l.tsv.gz" >"$w/tmp/vs-$l.tsv"
  python3 -I "$w/colldiff.py" "$w/tmp/base-$base.tsv" "$w/tmp/vs-$l.tsv" --rules >"$w/rules/$l.vs-$base.rules"
  head -1 "$w/rules/$l.vs-$base.rules"
  rm -f "$w/tmp/vs-$l.tsv"
done
rm -f "$w/tmp/base-$base.tsv"
