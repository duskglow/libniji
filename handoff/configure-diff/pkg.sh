#!/bin/sh
# CONFIGURE-DIFF: one source package in its own throw-away chroot (mmdebstrap --mode=unshare from the base tarball
# cdiff.sh made), inside.sh does the work; the results land in <run>/pkgs/<pkg>/.
# Usage: pkg.sh <run dir> <srcpkg> <version|->   (environment from cdiff.sh: CDIFF_BASE, CDIFF_STAGE, TMPDIR)
# A chroot that never produced /tmp/out (seen twice in 438 under load, cause not in the log then) is retried once;
# inside.sh's own output now goes to mmdebstrap.log so a repeat shows its cause.
set -u
R=$1
P=$2
V=$3
L=$R/pkgs/$P
for try in 1 2; do
    rm -rf "$L"; mkdir -p "$L"
    mmdebstrap --mode=unshare --variant=custom --skip=update,setup,cleanup,tar-in/mknod \
        --setup-hook="tar-in $CDIFF_BASE /" \
        --customize-hook="copy-in $CDIFF_STAGE/y /tmp" \
        --customize-hook='mkdir -p "$1/tmp/out"; chroot "$1" sh /tmp/y/inside.sh '"$P $V"' 2>&1 | tail -20; true' \
        --customize-hook="copy-out /tmp/out $L" \
        '' /dev/null > "$L/mmdebstrap.log" 2>&1
    rc=$?
    [ -s "$L/out/status.txt" ] && break
    echo "cdiff: $P try $try mmdebstrap rc=$rc without results" >> "$R/retries.txt"
done
echo "cdiff: mmdebstrap rc=$rc try=$try" >> "$L/out/status.txt" 2>/dev/null || echo "cdiff: mmdebstrap rc=$rc (no out)" > "$L/status-host.txt"
echo "pkg $P rc=$rc try=$try $(grep -h '^cdiff: done' "$L/out/status.txt" 2>/dev/null)"
