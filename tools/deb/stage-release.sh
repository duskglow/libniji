#!/bin/sh
# Stage release assets: copy .debs under the names GitHub publishes, write SHA256SUMS over those names.
#
# usage: stage-release.sh --out DIR [--head SHA] [--note TEXT] FILE.deb...
#
# GitHub does not allow `~` in a release asset name and publishes it with `.` in its place (issue #3:
# `liella6_0.0~snapshot...deb` became `liella6_0.0.snapshot...deb`, so a SHA256SUMS naming the `~` file
# failed `sha256sum -c` on the downloads). So each file is staged under its published name, and SHA256SUMS
# is computed over the staged names: `sha256sum -c SHA256SUMS` passes on exactly what is uploaded. The
# package versions inside the .debs are not touched. A name with a character outside A-Z a-z 0-9 . _ + -
# is refused (GitHub rewrites other characters too, and the published name would differ from the staged
# one). MANIFEST is not an asset: it maps each staged name to its package and Debian version, and names
# the build host.
set -eu

out= head= note=
while [ $# -gt 0 ]; do
    case $1 in
    --out) out=$2; shift 2 ;;
    --head) head=$2; shift 2 ;;
    --note) note=$2; shift 2 ;;
    --) shift; break ;;
    -*) echo "stage-release: unknown option $1" >&2; exit 2 ;;
    *) break ;;
    esac
done
[ -n "$out" ] && [ $# -gt 0 ] || { echo "usage: stage-release.sh --out DIR [--head SHA] [--note TEXT] FILE.deb..." >&2; exit 2; }
[ ! -e "$out" ] || { echo "stage-release: $out exists; not overwriting" >&2; exit 1; }

# The published name of a file name: `~` becomes `.`; anything outside the safe set is an error.
published() {
    p=$(printf '%s' "$1" | tr '~' '.')
    case $p in
    '' | *[!A-Za-z0-9._+-]*) echo "stage-release: '$1' has a character GitHub rewrites in asset names" >&2; return 1 ;;
    esac
    printf '%s\n' "$p"
}

# Check every name before anything is created, and that two files do not collapse into one name.
seen=
for f in "$@"; do
    [ -f "$f" ] || { echo "stage-release: $f is not a file" >&2; exit 1; }
    p=$(published "$(basename "$f")") || exit 1
    case " $seen " in *" $p "*) echo "stage-release: two files stage as $p" >&2; exit 1 ;; esac
    seen="$seen $p"
done

mkdir -p "$out"
for f in "$@"; do
    cp "$f" "$out/$(published "$(basename "$f")")"
done

cd "$out"
sha256sum -- *.deb > SHA256SUMS
sha256sum -c SHA256SUMS >/dev/null
{
    echo "Liella release assets (staged, not published)"
    [ -z "$head" ] || echo "head: $head"
    [ -z "$note" ] || echo "$note"
    echo "built: $(hostname -s), $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "Asset names use '.' where the Debian version has '~' (GitHub does not allow '~' in asset names)."
    echo
    printf '%-14s %-36s %-6s %10s  %s\n' package version arch bytes asset
    for f in *.deb; do
        printf '%-14s %-36s %-6s %10s  %s\n' "$(dpkg-deb -f "$f" Package)" "$(dpkg-deb -f "$f" Version)" \
            "$(dpkg-deb -f "$f" Architecture)" "$(stat -c %s "$f")" "$f"
    done
    echo
    echo "Verify: sha256sum -c SHA256SUMS"
} > MANIFEST
echo "stage-release: staged $# files in $out"
