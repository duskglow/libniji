#!/bin/sh
# CI-ENV-AUDIT (run 2): from-scratch vs incremental cost of the gate's build steps, in one role, sequentially.
# cold:   the role's target/ emptied (cargo cache and products gone).
# noop:   nothing changed.
# niji:   only target/niji and target/niji-test removed (what the CI now does before each gate).
# touch:  one leaf source file's mtime bumped (a train that changes one crate).
# Each phase: products (cargo xtask so), then the workspace test binaries (cargo test --workspace --no-run).
# Wall from date +%s.%N; disk from du of the resolved target directory (target/ is a symlink to RAM).
set -u
cd "$HOME/w/n1" || exit 2
. "$HOME/.cargo/env"
TD=$(readlink -f target)
O="$HOME/work/ci-env-audit"
ts() { date +%s.%N; }
dus() { du -sb "$TD" 2>/dev/null | cut -f1; }
fail=0
phase() {
  name=$1
  echo "=== phase $name start $(date -u +%H:%M:%S) target_bytes=$(dus)"
  t0=$(ts)
  cargo xtask so > "$O/m2-$name-so.out" 2>&1; rc1=$?
  t1=$(ts)
  cargo test --workspace --no-run > "$O/m2-$name-tests.out" 2>&1; rc2=$?
  t2=$(ts)
  echo "MEASURE phase=$name so_rc=$rc1 so_s=$(echo "$t1 - $t0" | bc) tests_rc=$rc2 tests_s=$(echo "$t2 - $t1" | bc) total_s=$(echo "$t2 - $t0" | bc) target_bytes_after=$(dus)"
  [ $rc1 -eq 0 ] && [ $rc2 -eq 0 ] || fail=1
}
echo "target=$TD"
find "$TD" -mindepth 1 -maxdepth 1 -exec rm -rf {} +
phase cold
du -sb "$TD"/* 2>/dev/null | sort -rn | head -12
phase noop
rm -rf "$TD/niji" "$TD/niji-test"
phase niji
touch crates/core/src/ctype.rs
phase touch
echo "=== done $(date -u +%H:%M:%S) fail=$fail"
exit $fail
