#!/bin/sh
# CI-ENV-AUDIT: file-access and exec trace of the gate's product build and of the product suites that
# went red on host or workspace state. Diagnostic only: test results under strace are not a verdict.
set -u
cd "$HOME/w/n1" || exit 2
. "$HOME/.cargo/env"
T=/dev/shm/claude/ci-env-audit/trace-1
mkdir -p "$T"
env | sed 's/=.*//' | sort > "$T/env-names.txt"
ST="strace -f -qq -s 256 -e signal=none -e trace=execve,open,openat,openat2,stat,lstat,newfstatat,statx,access,faccessat,faccessat2,readlink,readlinkat,mkdir,mkdirat,unlink,unlinkat,rename,renameat,renameat2"
echo "=== so start $(date -u +%H:%M:%S)"
$ST -o "$T/so.trace" cargo xtask so > "$T/so.out" 2>&1; echo "so rc=$?"
echo "=== tests start $(date -u +%H:%M:%S)"
$ST -o "$T/tests.trace" cargo xtask affected --only niji::locale_products,niji::catalogs_products,niji::deb_manifest,niji::hot_text_products,niji::dns_products,niji::heap_meta_products,niji::printf_corpus_products,niji::s07_locale_clauses,niji::locale_data --run > "$T/tests.out" 2>&1; echo "tests rc=$?"
echo "=== summarise $(date -u +%H:%M:%S)"
python3 -I "$HOME/work/ci-env-audit/summ.py" "$HOME/w/n1" "$T/so.trace" > "$HOME/work/ci-env-audit/summ-so.txt"
python3 -I "$HOME/work/ci-env-audit/summ.py" "$HOME/w/n1" "$T/tests.trace" > "$HOME/work/ci-env-audit/summ-tests.txt"
grep -E '^(step|suites|FAILED|affected)' "$T/tests.out" | head -40
ls -la "$T"
echo "=== done $(date -u +%H:%M:%S)"
