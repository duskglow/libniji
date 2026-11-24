#!/bin/bash
# LD-START-MAPS: build the role's release products (NIJI_FULL_PROFILE=1), keep a copy under ~/work/ld-start-maps/TAG
# usage: build.sh TAG
set -e
. ~/.cargo/env
TAG=$1
W=~/work/ld-start-maps
cd ~/w/s4
echo "tree: $(git rev-parse HEAD 2>/dev/null || true)"
NIJI_FULL_PROFILE=1 cargo xtask so > "$W/so-$TAG.log" 2>&1 || { tail -30 "$W/so-$TAG.log"; exit 1; }
rm -rf "$W/$TAG"; mkdir -p "$W/$TAG"
cp target/niji/ld-linux-x86-64.so.2 target/niji/libc.so.6 target/niji/niji-crt1.o target/niji/libc_nonshared.a "$W/$TAG/"
sha256sum "$W/$TAG"/* > "$W/$TAG/SHA256SUMS"
cat "$W/$TAG/SHA256SUMS"
