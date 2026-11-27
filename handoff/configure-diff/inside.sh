#!/bin/sh
# CONFIGURE-DIFF: runs INSIDE one throw-away trixie chroot (pkg.sh starts it) for one source package.
# Usage: inside.sh <srcpkg> <version|->
# Steps: apt-get update; build-dep and source (the version asked for, else trixie's); configure three times in the
# same directory path: stockA and stockB on trixie's libc (the stock-vs-stock control), then libniji's system
# flavour (libc6, libc-bin, libc6-dev, libc-dev-bin from /tmp/y/debs) is installed over it and the configure step
# runs again as "niji". After each run the files that carry probe answers (config.log, generated headers, perl's
# config.sh, meson-log.txt, CMakeCache.txt) are tarred to /tmp/out/<side>.tar for extract.py.
# The configure step is the Debian build's own (cfgrules.sh under dpkg-buildpackage -T, see there).
# Special case gcc-14: libstdc++-v3's configure runs standalone with the system g++ (stage-1 libstdc++ probes).
set -u
P=$1
V=$2
O=/tmp/out
mkdir -p "$O" /tmp/src
st() { echo "cdiff: $*" >> "$O/status.txt"; }
export DEBIAN_FRONTEND=noninteractive
T0=$(date +%s)
apt-get update > "$O/update.log" 2>&1
st "update rc=$?"

if [ "$P" = gcc-14 ]; then
    # Only the upstream tree is needed: no build-deps beyond build-essential's g++.
    : > "$O/bd.log"; st "build-dep rc=0 skipped (gcc-14 libstdc++-v3 standalone)"
else
    bv=-
    if [ "$V" != - ] && apt-get build-dep -y "$P=$V" > "$O/bd.log" 2>&1; then bv=$V
    else apt-get build-dep -y "$P" >> "$O/bd.log" 2>&1 && bv=trixie; fi
    st "build-dep rc=$? version=$bv"
fi
sv=-
cd /tmp/src || exit 1
if [ "$V" != - ] && apt-get source "$P=$V" > "$O/src.log" 2>&1; then sv=$V
else apt-get source "$P" >> "$O/src.log" 2>&1 && sv=trixie; fi
D=$(ls -d /tmp/src/*/ 2>/dev/null | head -1); D=${D%/}; D=${D##*/}
st "source rc=$? version=$sv dir=$D got=$(sed -n 's/^Version: //p' /tmp/src/*.dsc 2>/dev/null | head -1)"
[ -n "$D" ] || { st "NOSOURCE"; exit 0; }

if [ "$P" = gcc-14 ]; then
    tb=$(find "/tmp/src/$D" -maxdepth 1 -name 'gcc-*.tar.*' | head -1)
    mkdir -p /tmp/gsrc && tar -C /tmp/gsrc -xf "$tb" 2>> "$O/src.log"
    G=$(ls -d /tmp/gsrc/*/ | head -1); G=${G%/}
    st "gcc upstream tarball=${tb##*/} tree=$G"
fi

names='\( -name config.log -o -name "*.h" -o -name config.sh -o -name config.cache -o -name meson-log.txt -o -name CMakeCache.txt \)'
side() {
    s=$1
    rm -rf /tmp/w; mkdir -p /tmp/w
    if [ "$P" = gcc-14 ]; then
        mkdir -p /tmp/w/libstdc++-build; W=/tmp/w/libstdc++-build
        touch /tmp/stamp; sleep 1
        (cd "$W" && timeout 1500 "$G/libstdc++-v3/configure" --build=x86_64-linux-gnu --host=x86_64-linux-gnu \
            --disable-multilib --enable-languages=c++ CC=gcc-14 CXX=g++-14 > "$O/cfg-$s.log" 2>&1)
        r=$?
    else
        cp -a "/tmp/src/$D" /tmp/w/; W=/tmp/w/$D
        touch /tmp/stamp; sleep 1
        (cd "$W" && DEB_BUILD_OPTIONS="nocheck nodoc parallel=4" timeout 1500 dpkg-buildpackage -d -nc -uc -us \
            --rules-file=/tmp/y/cfgrules.sh -T cdiff-configure > "$O/cfg-$s.log" 2>&1)
        r=$?
    fi
    n=$(cd "$W" && eval "find . -newer /tmp/stamp -type f $names -size -40M" | LC_ALL=C sort | tee "$O/files-$s.txt" | wc -l)
    (cd "$W" && tar -cf "$O/$s.tar" -T "$O/files-$s.txt")
    st "configure $s rc=$r files=$n secs=$(( $(date +%s) - T0 ))"
}
side stockA
side stockB
dpkg-query -W libc6 libc6-dev gcc g++ debhelper > "$O/stock-versions.txt" 2>&1
dpkg -i --force-downgrade --force-depends /tmp/y/debs/*.deb > "$O/swap.log" 2>&1
st "swap rc=$? $(dpkg-query -W -f '${Package}=${Version} ' libc6 libc-bin libc6-dev libc-dev-bin 2>&1)"
side niji
st "done secs=$(( $(date +%s) - T0 ))"
