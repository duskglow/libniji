#!/bin/sh
# CONFIGURE-DIFF driver: configure every package of a list against stock trixie and against libniji's system
# flavour, extract every probe answer and diff them. See README.md.
# Usage: cdiff.sh -m <mirror url> -k <keyring> -d <libniji system debs dir> -o <run dir> [-j <parallel>] <pkglist>
#   pkglist: lines "<source package> <version|->"; "-" = trixie's version. mklist.sh writes Aqours' stage-2 list.
# Output: <run>/pkgs/<pkg>/out/{status.txt,*.log,stockA.tar,stockB.tar,niji.tar}, <run>/probes/<pkg>.<side>.tsv,
#         <run>/diff.tsv (every differing probe), <run>/summary.txt (per package: compared, noisy, differing).
set -u
H=$(cd "$(dirname "$0")" && pwd)
J=8; M=; K=; D=; R=
while getopts m:k:d:o:j: o; do case $o in m) M=$OPTARG;; k) K=$OPTARG;; d) D=$OPTARG;; o) R=$OPTARG;; j) J=$OPTARG;;
    *) echo "usage: cdiff.sh -m mirror -k keyring -d debdir -o rundir [-j N] pkglist" >&2; exit 2;; esac; done
shift $((OPTIND - 1))
LIST=${1:?pkglist}
[ -n "$M" ] && [ -n "$K" ] && [ -n "$D" ] && [ -n "$R" ] || { echo "cdiff: -m -k -d -o are required" >&2; exit 2; }
mkdir -p "$R/tmp" "$R/stage/y/debs" "$R/pkgs" "$R/probes"
R=$(cd "$R" && pwd)
export TMPDIR=$R/tmp
cp "$H/inside.sh" "$H/cfgrules.sh" "$R/stage/y/"
cp "$D"/libc6_*.deb "$D"/libc-bin_*.deb "$D"/libc6-dev_*.deb "$D"/libc-dev-bin_*.deb "$R/stage/y/debs/" || exit 1
chmod -R a+rX "$R/stage"; chmod 0755 "$R/stage/y/cfgrules.sh"
ls "$R/stage/y/debs" | sed 's/^/cdiff: libniji deb /'
if [ ! -s "$R/base.tar" ]; then
    mmdebstrap --mode=unshare --variant=apt --keyring="$K" \
        --include=build-essential,dpkg-dev,fakeroot,file,debhelper,ca-certificates \
        trixie "$R/base.tar" "deb $M trixie main" "deb-src $M trixie main" > "$R/base.log" 2>&1 \
        || { echo "cdiff: base tarball failed"; tail -5 "$R/base.log"; exit 1; }
fi
export CDIFF_BASE=$R/base.tar CDIFF_STAGE=$R/stage
grep -v '^#' "$LIST" | awk 'NF { print $1, ($2 == "" ? "-" : $2) }' > "$R/pkglist"
echo "cdiff: $(wc -l < "$R/pkglist") packages, $J at a time"
xargs -P "$J" -L 1 sh "$H/pkg.sh" "$R" < "$R/pkglist" > "$R/pkgs.txt" 2>&1
python3 "$H/extract.py" "$R"
python3 "$H/diff.py" "$R"
