#!/bin/sh
# tools/deb/version.sh FLAVOUR EPOCH COMMIT: print the Debian version of a libniji package build.
#
#   coexist  0.0~snapshot<YYYYMMDD>.<n>+<12-hex commit>
#   system   <abi>+niji<YYYYMMDD>.<n>-<rev>
#
# <YYYYMMDD> is the UTC date of EPOCH, <n> is NIJI_DEB_SERIAL (default 1: the snapshot of the day,
# the same input for both flavours), <rev> NIJI_DEB_REVISION (default 1). The serial sits before the
# commit so that two snapshots of one day order by serial, not by commit hash. A `.` sorts above the
# `+` of the earlier serial-less coexist versions (0.0~snapshot<date>+<commit>), so every serial
# version sorts above every one of those. tools/deb/version.sh is run by build.sh and by
# crates/core/tests/deb_version_order.rs.
set -eu
export LC_ALL=C TZ=UTC
[ $# -eq 3 ] || { echo "usage: tools/deb/version.sh coexist|system EPOCH COMMIT" >&2; exit 2; }
flavour=$1
epoch=$2
commit=$(printf '%.12s' "$3")
serial=${NIJI_DEB_SERIAL:-1}
case $serial in
    '' | *[!0-9]* | 0*) echo "version.sh: NIJI_DEB_SERIAL=$serial: a positive number without leading zeros" >&2; exit 2 ;;
esac
day=$(date -u -d "@$epoch" +%Y%m%d)
case $flavour in
    coexist) printf '%s\n' "0.0~snapshot$day.$serial+$commit" ;;
    system) printf '%s\n' "${NIJI_DEB_ABI:-2.39}+niji$day.$serial-${NIJI_DEB_REVISION:-1}" ;;
    *) echo "version.sh: $flavour: coexist or system" >&2; exit 2 ;;
esac
