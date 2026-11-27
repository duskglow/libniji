#!/bin/sh
# CONFIGURE-DIFF: dpkg-buildpackage --rules-file for "configure only" (inside.sh: dpkg-buildpackage -T cdiff-configure).
# dpkg-buildpackage sets up the build environment as for a real build and then calls this script instead of
# debian/rules. For a dh package the "build" sequence is listed with DH_NO_ACT=1 and run up to and including the
# configure step (execute_before_/override_/execute_after_dh_auto_configure or dh_auto_configure), so the package's
# own configure flags and overrides apply. A package without dh: the first debian/rules target whose name says
# configure (configure, configure-stamp, config.status, ...) runs; none found: NOCFG.
set -u
if grep -Eq '^[[:space:]]*dh[[:space:]]+(\$@|\$\(@\)|build|\$\*)|^[[:space:]]*dh[[:space:]].*\$@' debian/rules; then
    seq=$(DH_NO_ACT=1 debian/rules build 2>/dev/null | sed -n 's/^   //p')
    last=$(printf '%s\n' "$seq" | grep -n 'dh_auto_configure' | tail -1 | cut -d: -f1)
    if [ -n "$last" ]; then
        printf '%s\n' "$seq" | head -n "$last" > /tmp/cdiff-seq.txt
        echo "cdiff-cfgrules: dh sequence up to configure:"; sed 's/^/cdiff-cfgrules:   /' /tmp/cdiff-seq.txt
        while IFS= read -r c; do
            echo "cdiff-cfgrules: run $c"
            sh -c "$c" || { echo "cdiff-cfgrules: FAILED rc=$? $c"; exit 1; }
        done < /tmp/cdiff-seq.txt
        exit 0
    fi
    echo "cdiff-cfgrules: dh package, no configure step in the build sequence"
fi
t=$(make -f debian/rules -qp 2>/dev/null | sed -n 's/^\([A-Za-z0-9_./$()-]*config[A-Za-z0-9_./-]*\):.*/\1/p' \
    | grep -E '(^|[/-])(configure|config\.status|configure-stamp|stamp-configure|config-stamp)$|configure' \
    | grep -v -E '^(Makefile|debian/rules)$|\$' | awk '{ print length($0) " " $0 }' | sort -n | cut -d' ' -f2- | head -1)
if [ -n "$t" ]; then
    echo "cdiff-cfgrules: non-dh target $t"
    exec debian/rules "$t"
fi
echo "cdiff-cfgrules: NOCFG"
exit 3
