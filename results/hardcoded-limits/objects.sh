#!/bin/bash
# AUDIT-HARDCODED-LIMITS ranking evidence: how many shared objects real programs on a host load.
# 1. Start-time: the ldd closure (lines with "=>" plus the loader) of every ELF program in /usr/bin
#    and /usr/sbin and every shared object in /usr/lib/x86_64-linux-gnu; the 15 largest, and how
#    many exceed 64 / 256.
# 2. Run-time: objects a python3 process maps after importing every stdlib extension module.
set -u
tmp=$(mktemp)
for f in /usr/bin/* /usr/sbin/* /usr/lib/x86_64-linux-gnu/*.so*; do
  [ -f "$f" ] && [ ! -L "$f" ] || continue
  head -c 4 "$f" 2>/dev/null | grep -q 'ELF' || continue
  n=$(timeout 10 ldd "$f" 2>/dev/null | grep -c '=>')
  [ "$n" -gt 0 ] && echo "$((n + 1)) $f"
done | sort -rn > "$tmp"
echo "start-time objects (ldd closure + loader), files examined: $(wc -l < "$tmp")"
echo "  over 64: $(awk '$1 > 64' "$tmp" | wc -l); over 256: $(awk '$1 > 256' "$tmp" | wc -l)"
head -15 "$tmp" | sed 's/^/  /'
rm -f "$tmp"
timeout 120 python3 - <<'EOF'
import importlib, pkgutil, sys, os
mods = [m.name for m in pkgutil.iter_modules() if m.module_finder.path.endswith('lib-dynload')]
ok = 0
for m in mods:
    try:
        importlib.import_module(m); ok += 1
    except Exception:
        pass
objs = {l.split()[-1] for l in open('/proc/self/maps') if '.so' in l.split()[-1]}
print(f"python3 {sys.version.split()[0]}: imported {ok} of {len(mods)} stdlib extension modules; shared objects mapped: {len(objs)}")
EOF
