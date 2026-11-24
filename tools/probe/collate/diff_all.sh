#!/bin/sh
# Diff every surveyed locale's order against a root locale's.
# usage: diff_all.sh <workdir> <root-name> <jobs>
# writes <workdir>/rules/<name>.rules, <workdir>/diffs.tsv, <workdir>/groups.tsv
w=$1; root=$2; j=$3
mkdir -p "$w/rules" "$w/tmp"
gzip -dc "$w/raw/$root.tsv.gz" >"$w/tmp/root.tsv"
ls "$w/raw" | sed 's/\.tsv\.gz$//' | xargs -P "$j" -I{} sh -c \
  'gzip -dc "$1/raw/$2.tsv.gz" >"$1/tmp/$2.tsv" && python3 -I "$1/colldiff.py" "$1/tmp/root.tsv" "$1/tmp/$2.tsv" --rules >"$1/rules/$2.rules"; rm -f "$1/tmp/$2.tsv"' _ "$w" {}
for f in "$w"/rules/*.rules; do head -1 "$f"; done >"$w/diffs.tsv"
for f in "$w"/raw/*.tsv.gz; do printf '%s\t%s\n' "$(gzip -dc "$f" | md5sum | cut -c1-12)" "$(basename "$f" .tsv.gz)"; done |
  sort >"$w/groups.tsv"
echo "diffs: $(wc -l <"$w/diffs.tsv") distinct orders: $(cut -f1 "$w/groups.tsv" | sort -u | wc -l)"
