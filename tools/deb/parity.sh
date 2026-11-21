#!/bin/sh
# tools/deb/parity.sh: rerun the package parity audit (plan/package-parity-audit.md).
#
#   tools/deb/parity.sh WORK OURS-DEBS OUT [SAMPLE]
#
# WORK: a new scratch directory (indices and the reference .debs land there, about 70 MB; the
# ranking scan streams each sampled package through a temporary directory under WORK and deletes
# it). OURS-DEBS: the system packages (tools/deb/build.sh --flavour system). OUT: the directory
# for the inventories and tables, results/package-parity in the tree. SAMPLE: packages in the ELF
# scan (default 4000; seed fixed in parity.py, so the same indices draw the same sample).
# Needs network access to deb.debian.org and security.debian.org, dpkg-deb, readelf and ar.
set -eu
[ $# -ge 3 ] || { echo "usage: $0 WORK OURS-DEBS OUT [SAMPLE]" >&2; exit 2; }
work=$1 ours=$2 out=$3 sample=${4:-4000}
p=$(dirname "$0")/parity.py
mkdir -p "$work/scan" "$out"
python3 "$p" index "$work" --suite trixie
cp "$work/index-meta.json" "$out/index-meta.json"
python3 "$p" fetch-ref "$work" --suite trixie
python3 "$p" inventory "$out/ref" "$work"/ref-debs/*.deb
python3 "$p" inventory "$out/ours" "$ours"/*.deb
python3 "$p" compare "$out"
python3 "$p" rank "$work" "$out" --sample "$sample" --jobs 16 --tmp "$work/scan"
wc -l "$out"/*.tsv
