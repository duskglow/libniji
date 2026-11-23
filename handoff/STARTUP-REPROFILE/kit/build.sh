#!/bin/bash
# STARTUP-REPROFILE: build next's products on the role (fast profile, as LD-LOOKUP-COST's kit), keep a copy
# usage: build.sh TAG   (release products, the shipped profile: NIJI_FULL_PROFILE=1 build.sh release)
set -e
. ~/.cargo/env
TAG=$1
W=~/work/startup-reprofile
cd ~/w/r1
echo "sync: $(cat .sync-tree 2>/dev/null || ls -a | grep -m1 sync)"
cargo xtask so > "$W/so-$TAG.log" 2>&1 || { tail -30 "$W/so-$TAG.log"; exit 1; }
rm -rf "$W/$TAG"; mkdir -p "$W/$TAG"
cp target/niji/ld-linux-x86-64.so.2 target/niji/libc.so.6 target/niji/niji-crt1.o target/niji/libc_nonshared.a "$W/$TAG/"
cp crates/core/tests/oracle/exec_instr.c "$W/"
sha256sum "$W/$TAG"/* > "$W/$TAG/SHA256SUMS"
cat "$W/$TAG/SHA256SUMS"
