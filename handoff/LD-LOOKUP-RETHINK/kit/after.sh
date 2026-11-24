#!/bin/bash
# LD-LOOKUP-RETHINK: the start-up checks (cargo xtask test; release products and hot_text_products) once the gate
# job has ended, so one build runs in the role at a time. usage: after.sh GATE_LABEL
set -e
while true; do rc=0; ~/bin/jobwait "$1" > /dev/null || rc=$?; [ "$rc" = 3 ] || break; done
echo "gate $1 ended (jobwait rc $rc)"
. ~/.cargo/env
~/bin/gate 16G sh -c "cd ~/w/sh2 && cargo xtask test"
echo "xtask test passed"
~/bin/gate 16G sh -c "cd ~/w/sh2 && NIJI_FULL_PROFILE=1 cargo xtask so && NIJI_FULL_PROFILE=1 cargo test -p niji --test hot_text_products"
echo "startup checks done"
