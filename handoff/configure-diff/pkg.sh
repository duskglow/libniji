#!/bin/sh
# CONFIGURE-DIFF: one source package in its own throw-away chroot (mmdebstrap --mode=unshare from the base tarball
# cdiff.sh made), inside.sh does the work; the results land in <run>/pkgs/<pkg>/.
# Usage: pkg.sh <run dir> <srcpkg> <version|->   (environment from cdiff.sh: CDIFF_BASE, CDIFF_STAGE, TMPDIR)
set -u
R=$1
P=$2
V=$3
L=$R/pkgs/$P
rm -rf "$L"; mkdir -p "$L"
mmdebstrap --mode=unshare --variant=custom --skip=update,setup,cleanup,tar-in/mknod \
    --setup-hook="tar-in $CDIFF_BASE /" \
    --customize-hook="copy-in $CDIFF_STAGE/y /tmp" \
    --customize-hook='chroot "$1" sh /tmp/y/inside.sh '"$P $V"' > /dev/null 2>&1; true' \
    --customize-hook="copy-out /tmp/out $L" \
    '' /dev/null > "$L/mmdebstrap.log" 2>&1
rc=$?
echo "cdiff: mmdebstrap rc=$rc" >> "$L/out/status.txt" 2>/dev/null || echo "cdiff: mmdebstrap rc=$rc (no out)" > "$L/status-host.txt"
echo "pkg $P rc=$rc $(grep -h '^cdiff: done' "$L/out/status.txt" 2>/dev/null)"
