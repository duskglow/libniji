#!/bin/sh
# LAYOUT-TOOL-BOUND-SHARED: build.sh <tag> [<sha-to-merge>]
# A RAM clone of next (NEXT below) with <sha> merged onto it as CI merges a branch onto next (nothing pushed), fast
# products (`cargo xtask so`, CI's profile), the coreutils-shaped bound test itself, then pf.py's fault attribution
# into $R/out. setup.sh must have filled $R/mirror.git.
set -eu
R=/mnt/zram/claude/LAYOUT-TOOL-BOUND-SHARED
W=$HOME/work/LAYOUT-TOOL-BOUND-SHARED
NEXT=${LTB_NEXT:-955c18b87}
tag=$1; ref=${2:-}
. "$HOME/.cargo/env"
t=$R/$tag
git clone -q "$R/mirror.git" "$t"
cd "$t"
git checkout -q --detach "$NEXT"
if [ -n "$ref" ]; then
    git -c user.name=ltb -c user.email=ltb@invalid merge -q --no-edit "$ref"
fi
echo "ltb: $tag: head $(git rev-parse --short=12 HEAD) next $NEXT ref ${ref:-none}"
rc=0; cargo xtask so || rc=$?
echo "ltb: $tag: so rc=$rc"
[ "$rc" = 0 ] || exit "$rc"
echo "ltb: $tag: sizes $(stat -c '%n=%s' target/liella/libc.a target/liella/libc.so.6 | tr '\n' ' ')"
trc=0; cargo test -q -p liella --test hot_text_products init_coreutils_shaped_program_stays_within_its_page_fault_bound -- --nocapture || trc=$?
echo "ltb: $tag: tool bound test rc=$trc"
python3 "$W/pf.py" "$t" "$tag" "$R/out" tool
exit "$trc"
