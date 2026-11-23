#!/bin/sh
# Collation survey over every UTF-8 locale the reference lists as supported.
# usage: survey_all.sh <workdir> <ramdir> <jobs>
w=$1; ram=$2; j=$3
mkdir -p "$w/logs" "$w/raw" "$ram"
grep -E ' UTF-8$' /usr/share/i18n/SUPPORTED | sed 's/ .*//; s/\.UTF-8//' | sort -u >"$w/locales.txt"
echo "locales: $(wc -l <"$w/locales.txt")"
xargs -P "$j" -n 1 sh "$w/survey_one.sh" "$w" "$ram" <"$w/locales.txt"
cat "$w"/logs/*.row >"$w/rows.tsv"
cat "$w"/logs/*.summary >"$w/summaries.txt"
echo "rows: $(wc -l <"$w/rows.tsv") localedef-failed: $(grep -vc 'localedef_rc=0' "$w/rows.tsv") probe-failed: $(grep -vc 'probe_rc=0' "$w/rows.tsv")"
