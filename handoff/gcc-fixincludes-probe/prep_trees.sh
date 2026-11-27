#!/bin/bash
# Extract each libc6-dev deb into its own tree: $W/trees/<name>/usr/include.
set -euo pipefail
W=~/work/gcc-fixincludes-probe
mkdir -p "$W/trees" "$W/stockdeb"
for d in "$W"/debs/*/; do
  n=$(basename "$d"); n=${n#snapshot-}
  deb=$(ls "$d"libc6-dev_*.deb | head -1)
  [ -d "$W/trees/$n" ] || dpkg-deb -x "$deb" "$W/trees/$n"
done
# Stock reference: the installed Ubuntu libc6-dev 2.39 package (a fresh copy, so only its own files are scanned).
cd "$W/stockdeb"
[ -f libc6-dev_*.deb ] 2>/dev/null || apt-get download libc6-dev
[ -d "$W/trees/stock-2.39" ] || dpkg-deb -x libc6-dev_*.deb "$W/trees/stock-2.39"
# Debian trixie's libc6-dev (the libc of Aqours' stage 0 gcc-14 build).
v=2.41-12+deb13u4
[ -f "libc6-dev_${v}_amd64.deb" ] || curl -sSfO "https://deb.debian.org/debian/pool/main/g/glibc/libc6-dev_${v}_amd64.deb" || \
  curl -sSfO "https://security.debian.org/debian-security/pool/updates/main/g/glibc/libc6-dev_${v}_amd64.deb" || echo "no trixie deb"
[ -d "$W/trees/trixie-2.41" ] || { [ -f "libc6-dev_${v}_amd64.deb" ] && dpkg-deb -x "libc6-dev_${v}_amd64.deb" "$W/trees/trixie-2.41"; }
ls "$W/trees"
for t in "$W"/trees/*/; do echo "$(basename "$t") $(find "$t/usr/include" -name '*.h' | wc -l) headers"; done
