#!/bin/bash
# LD-LOOKUP-RETHINK: the counts of the final head. Once the startup job (after.sh: its last step builds the
# full-profile products, the profile step.sh counted) has ended green, copy target/niji and count it with
# meas2.sh (no trace), then compare the products with the ones d1b counted. usage: counts2.sh STARTUP_LABEL TAG
set -e
while true; do rc=0; ~/bin/jobwait "$1" > /dev/null || rc=$?; [ "$rc" = 3 ] || break; done
echo "startup $1 ended (jobwait rc $rc)"
[ "$rc" = 0 ] || { echo "startup job did not pass: no counts"; exit 1; }
K=~/work/ld-lookup-rethink; TAG=$2
cd ~/w/sh2
echo "tree: $(head -1 .sync-tree 2>/dev/null)"
mkdir -p "$K/$TAG"
cp target/niji/ld-linux-x86-64.so.2 target/niji/libc.so.6 target/niji/niji-crt1.o target/niji/libc_nonshared.a "$K/$TAG/"
bash "$K/meas2.sh" "$K/$TAG" "$K/out-$TAG"
if cmp -s <(cut -d' ' -f1 "$K/out-d1b/products.sha256") <(cut -d' ' -f1 "$K/out-$TAG/products.sha256"); then
  echo "products: identical to the ones d1b counted"
else
  echo "products: differ from the ones d1b counted"
fi
echo "counts $TAG done"
