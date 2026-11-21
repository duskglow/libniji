#!/bin/bash
# LAB-ONLY: this script drives the project's own test hosts and is not needed to build or test libniji.
# Self-test of the products profile and cache key of validation/wl_lock.sh (WL-PREP-RELEASE-PROFILE; no spec clause:
# measurement tooling). The workload gate and the per-call counts must measure what ships, the release profile:
#   1. wl_key: <sha>-release by default, <sha> (the old key) with WL_PROFILE=fast, an unknown profile refused;
#   2. wl_build_products builds with NIJI_FULL_PROFILE=1 and refuses a build that left fast products;
#   3. a fast set cached under the old key <sha> (no .profile marker) is never taken for release, and is kept;
#   4. a cached set of the other profile under a key is refused, never reused, rebuilt over or deleted;
#   5. WL_PROFILE=fast reads the old fast cache without building;
#   6. WL_LINK=dynamic (WL-DYNAMIC-KIT) keys apart (<sha>-release-dyn) and never takes a static set;
#   7. wl_dynlink.py: a SONAME-less copy, and a program's by-path DT_NEEDED retargeted in place (cc and readelf);
#   8. WL-DYNAMIC-GATE: with no WL_LINK the link is dynamic (the per-gate kit), the timing gate pins static, and one
#      build caches both links' sets (8b, with cc).
# Runs the real wl_lock.sh functions in a scratch HOME with a stand-in `cargo` that writes products and xtask's
# target/niji/.profile marker the way `cargo xtask so` does (fast unless NIJI_FULL_PROFILE is set and not 0).
# Seconds; no libniji build. `affected` runs it when wl_lock.sh or this file changes (VALIDATION_COVERED).
set -u
here=$(cd "$(dirname "$0")" && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
export HOME=$tmp/home
unset NIJI_FULL_PROFILE WL_PROFILE WL_LINK
stub=$tmp/stub
mkdir -p "$HOME/wl/src" "$HOME/wl/products" "$stub"
# bash -l (wl_build_products) may reset PATH from /etc/profile; the scratch HOME's .profile puts the stand-in first again.
printf 'PATH=%s:$PATH\n' "$stub" > "$HOME/.profile"
export PATH=$stub:$PATH
printf '%s\n' '#!/bin/sh' \
    '[ "$1 $2" = "xtask so" ] || { echo "stand-in cargo: unexpected $*" >&2; exit 2; }' \
    'echo build >> "$HOME/builds"' \
    'p=fast; case ${NIJI_FULL_PROFILE:-} in ""|0) ;; *) p=release ;; esac; [ -n "${STUB_PROFILE:-}" ] && p=$STUB_PROFILE' \
    'mkdir -p target/niji && for f in libc.a libm.a niji-crt1.o; do echo "$p" > target/niji/$f; done' \
    '[ -n "${STUB_SO:-}" ] && cp "$STUB_SO"/* target/niji/' \
    'echo "$p" > target/niji/.profile' > "$stub/cargo"
chmod +x "$stub/cargo"
. "$here/wl_lock.sh"
pass=0; fail=0
check() { if [ "$2" = "$3" ]; then pass=$((pass + 1)); else fail=$((fail + 1)); echo "FAIL $1: expected [$2] got [$3]"; fi; }
builds() { if [ -f "$HOME/builds" ]; then wc -l < "$HOME/builds" | tr -d ' '; else echo 0; fi; }
S=0123456789abcdef0123456789abcdef01234567
T=89abcdef0123456789abcdef0123456789abcdef
U=fedcba9876543210fedcba9876543210fedcba98
for s in $S $T $U; do mkdir -p "$HOME/wl/src/$s"; done

# 8 (first, before any WL_LINK is set). THE GATE BINDS THE DYNAMIC KIT (WL-DYNAMIC-GATE, owner ruling #6965, D-X.141):
# with no WL_LINK the link is dynamic and every key is the dynamic one; the timing gate pins static itself.
check "default link is dynamic (the per-gate kit)" dynamic "$(wl_link)"
check "default key is the dynamic one" "$S-release-dyn" "$(wl_key $S)"
check "default product file is the shared libc" libc.so.6 "$(wl_prodfile)"
check "timing gate pins the static kit" 1 "$(grep -c '^export WL_LINK=static$' "$here/wl_gate.sh")"

# Sections 1-5: the profile rules on the static set.
export WL_LINK=static
# 1. keys
check "default key" "$S-release" "$(wl_key $S)"
check "fast key" "$S" "$(WL_PROFILE=fast wl_key $S)"
check "release key" "$S-release" "$(WL_PROFILE=release wl_key $S)"
check "glibc variant key" glibc "$(wl_vkey glibc)"
check "commit variant key" "$S-release" "$(wl_vkey $S)"
WL_PROFILE=debug wl_key $S > /dev/null 2>&1; check "unknown profile refused (rc)" 2 "$?"

# 2 + 3. a legacy fast set under <sha> (cached before this change: no marker), then a release build of that sha
mkdir -p "$HOME/wl/products/$S" && echo legacy > "$HOME/wl/products/$S/libc.a" && echo legacy > "$HOME/wl/products/$S/libm.a"
wl_products_ok "$HOME/wl/products/$S" 2> /dev/null; check "legacy set is fast, not release (rc)" 2 "$?"
wl_build_products $S > "$tmp/b1.log" 2>&1; check "release build of a sha with a legacy fast cache (rc)" 0 "$?"
check "release build ran cargo once" 1 "$(builds)"
check "release products marker" release "$(cat "$HOME/wl/products/$S-release/.profile" 2>/dev/null)"
check "release libc.a built by the release profile" release "$(cat "$HOME/wl/products/$S-release/libc.a" 2>/dev/null)"
check "legacy fast cache kept" legacy "$(cat "$HOME/wl/products/$S/libc.a" 2>/dev/null)"
wl_build_products $S > /dev/null 2>&1; check "second release build reuses the cache (rc)" 0 "$?"
check "second release build ran no cargo" 1 "$(builds)"

# 5. the fast profile reads the legacy cache without building
WL_PROFILE=fast wl_build_products $S > /dev/null 2>&1; check "fast build reuses the legacy cache (rc)" 0 "$?"
check "fast reuse ran no cargo" 1 "$(builds)"
WL_PROFILE=fast wl_build_products $U > /dev/null 2>&1; check "fast build of a new sha (rc)" 0 "$?"
check "fast products marker" fast "$(cat "$HOME/wl/products/$U/.profile" 2>/dev/null)"
check "release key untouched by a fast build" no "$([ -e "$HOME/wl/products/$U-release" ] && echo yes || echo no)"

# 2b. a build that leaves products of the other profile (STUB_PROFILE: the stand-in ignores the switch) is refused
V=00112233445566778899aabbccddeeff00112233
mkdir -p "$HOME/wl/src/$V"
STUB_PROFILE=fast wl_build_products $V > "$tmp/b2.log" 2>&1; check "build that left fast products refused (rc)" 1 "$?"
check "refused build cached nothing" no "$([ -e "$HOME/wl/products/$V-release" ] && echo yes || echo no)"

# 4. the other profile under a key: refused, kept, not rebuilt
mkdir -p "$HOME/wl/products/$T-release" && for f in libc.a libm.a; do echo fast > "$HOME/wl/products/$T-release/$f"; done
echo fast > "$HOME/wl/products/$T-release/.profile"
n=$(builds)
wl_build_products $T > "$tmp/b4.log" 2>&1; check "fast set under a release key refused (rc)" 1 "$?"
check "refused set kept" fast "$(cat "$HOME/wl/products/$T-release/libc.a" 2>/dev/null)"
check "refused set not rebuilt" "$n" "$(builds)"
check "refusal names the profiles" 1 "$(grep -c "holds products of profile 'fast', this run wants release" "$tmp/b4.log")"

# 6. WL_LINK=dynamic (WL-DYNAMIC-KIT): its own key, never a static set taken for it, an unknown link refused
check "dynamic key" "$S-release-dyn" "$(WL_LINK=dynamic wl_key $S)"
check "dynamic fast key" "$S-dyn" "$(WL_LINK=dynamic WL_PROFILE=fast wl_key $S)"
check "static key named" "$S-release" "$(WL_LINK=static wl_key $S)"
check "glibc variant key under dynamic" glibc "$(WL_LINK=dynamic wl_vkey glibc)"
WL_LINK=shared wl_key $S > /dev/null 2>&1; check "unknown link refused (rc)" 2 "$?"
WL_LINK=dynamic wl_products_ok "$HOME/wl/products/$S-release" 2> /dev/null; check "static set is not a dynamic set (rc)" 1 "$?"

# 7. wl_dynlink.py on a real link (cc, readelf): a SONAME-less copy, then the program's absolute DT_NEEDED of that
# copy pointed at the runtime file, which keeps its SONAME; the interpreter checked.
if command -v cc > /dev/null && command -v readelf > /dev/null && command -v python3 > /dev/null; then
    D=$tmp/dyn
    mkdir -p "$D/lib"
    printf 'int wl_probe(void) { return 7; }\n' > "$tmp/lib.c"
    printf 'int wl_probe(void);\nint main(void) { return wl_probe(); }\n' > "$tmp/main.c"
    cc -shared -fPIC -Wl,-soname,libc.so.6 -o "$D/libc.so.6" "$tmp/lib.c"
    python3 "$here/wl_dynlink.py" bypath "$D/libc.so.6" "$D/lib/libc-by-path.so"; check "bypath (rc)" 0 "$?"
    check "bypath copy has no SONAME" 0 "$(readelf -d "$D/lib/libc-by-path.so" | grep -c SONAME)"
    check "runtime file keeps its SONAME" 1 "$(readelf -d "$D/libc.so.6" | grep -c 'SONAME.*\[libc.so.6\]')"
    cc -o "$tmp/prog" "$tmp/main.c" "$D/lib/libc-by-path.so" "-Wl,--dynamic-linker=$D/ld-linux-x86-64.so.2"
    check "link records the by-path copy" 1 "$(readelf -d "$tmp/prog" | grep -c "NEEDED.*\[$D/lib/libc-by-path.so\]")"
    cp "$tmp/prog" "$tmp/prog2"
    python3 "$here/wl_dynlink.py" retarget "$tmp/prog" "$D" > /dev/null; check "retarget (rc)" 0 "$?"
    check "retargeted DT_NEEDED is the runtime file" 1 "$(readelf -d "$tmp/prog" | grep -c "NEEDED.*\[$D/libc.so.6\]")"
    check "no by-path DT_NEEDED left" 0 "$(readelf -d "$tmp/prog" | grep -c 'by-path')"
    check "file size unchanged" "$(wc -c < "$tmp/prog2")" "$(wc -c < "$tmp/prog")"
    python3 "$here/wl_dynlink.py" retarget "$tmp/prog" "$D" > /dev/null 2>&1; check "nothing left to retarget refused (rc)" 1 "$?"
    python3 "$here/wl_dynlink.py" retarget "$tmp/prog2" "$tmp/elsewhere" > /dev/null 2>&1; check "other products directory refused (rc)" 1 "$?"
    # 8b. ONE BUILD PER SHA (WL-DYNAMIC-GATE): a dynamic build (the default link) also caches the static set, so the
    # static sweep of that sha runs no second `cargo xtask so`.
    SO=$tmp/stubso
    mkdir -p "$SO"
    cc -shared -fPIC -Wl,-soname,libc.so.6 -o "$SO/libc.so.6" "$tmp/lib.c"
    cc -shared -fPIC -Wl,-soname,libm.so.6 -o "$SO/libm.so.6" "$tmp/lib.c"
    for f in ld-linux-x86-64.so.2 libc_nonshared.a niji-crti.o niji-crtn.o; do echo stub > "$SO/$f"; done
    W=4455667788990011223344556677889900112233
    mkdir -p "$HOME/wl/src/$W"
    n=$(builds)
    (unset WL_LINK; STUB_SO=$SO wl_build_products $W > "$tmp/b8.log" 2>&1); check "dynamic build (default link) (rc)" 0 "$?"
    (unset WL_LINK; wl_products_ok "$HOME/wl/products/$W-release-dyn"); check "dynamic set cached" 0 "$?"
    WL_LINK=static wl_products_ok "$HOME/wl/products/$W-release"; check "the same build cached the static set" 0 "$?"
    WL_LINK=static wl_build_products $W > /dev/null 2>&1; check "static set of that sha (rc)" 0 "$?"
    check "one cargo build served both links" $((n + 1)) "$(builds)"
else
    echo "wl_lock_test: section 7 skipped: no cc, readelf or python3"
fi

echo "wl_lock_test: $pass passed, $fail failed"
[ $fail = 0 ]
