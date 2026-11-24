#!/bin/bash
# LD-LOOKUP-RETHINK: release products of the role's tree, kept as ~/work/ld-lookup-rethink/TAG, counted and traced
# (meas2.sh), then the workspace clippy. usage: step.sh TAG
set -e
. ~/.cargo/env
TAG=$1; K=~/work/ld-lookup-rethink
cd ~/w/sh2
echo "tree: $(cat .sync-tree 2>/dev/null | head -1)"
NIJI_FULL_PROFILE=1 cargo xtask so
mkdir -p "$K/$TAG"
cp target/niji/ld-linux-x86-64.so.2 target/niji/libc.so.6 target/niji/niji-crt1.o target/niji/libc_nonshared.a "$K/$TAG/"
bash "$K/meas2.sh" "$K/$TAG" "$K/out-$TAG" trace
cd ~/w/sh2 && cargo xtask clippy
echo "step $TAG done"
