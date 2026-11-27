#!/bin/sh
# CONFIGURE-DIFF: package list from Aqours' stage-2 result files plus the earlier snapshot stories' ybuild set.
# Usage: mklist.sh <aqours stage2 log dir> > pkglist      Lines: "<source package> <version>"
set -u
for f in "$1"/*.result; do
    b=${f##*/}; b=${b%.result}
    echo "${b%%_*} ${b#*_}"
done
# The 1127B ybuild set and perl/apt are stage-2 packages too; the gcc-14 entry runs libstdc++-v3's configure.
