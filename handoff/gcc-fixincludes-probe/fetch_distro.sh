#!/bin/bash
# Fetch distributions' gcc packaging files (build recipes only) and grep them for fixincludes handling.
set -u
D=~/work/gcc-fixincludes-probe/distro
mkdir -p "$D"
cd "$D"
fetch() { # name url
  if curl -sSfL --max-time 60 -o "$1" "$2"; then echo "got $1 ($(wc -l < "$1") lines) from $2"; else echo "FAIL $1 $2"; fi
}
fetch alpine-APKBUILD "https://gitlab.alpinelinux.org/alpine/aports/-/raw/master/main/gcc/APKBUILD"
fetch void-template "https://raw.githubusercontent.com/void-linux/void-packages/master/srcpkgs/gcc/template"
fetch buildroot-gcc.mk "https://gitlab.com/buildroot.org/buildroot/-/raw/master/package/gcc/gcc.mk"
fetch buildroot-gcc-final.mk "https://gitlab.com/buildroot.org/buildroot/-/raw/master/package/gcc/gcc-final/gcc-final.mk"
fetch ctng-gcc.sh "https://raw.githubusercontent.com/crosstool-ng/crosstool-ng/master/scripts/build/cc/gcc.sh"
fetch gentoo-toolchain.eclass "https://gitweb.gentoo.org/repo/gentoo.git/plain/eclass/toolchain.eclass"
fetch fedora-gcc.spec "https://src.fedoraproject.org/rpms/gcc/raw/rawhide/f/gcc.spec"
fetch arch-PKGBUILD "https://gitlab.archlinux.org/archlinux/packaging/packages/gcc/-/raw/main/PKGBUILD"
fetch chimera-template "https://raw.githubusercontent.com/chimera-linux/cports/master/main/gcc/template.py"
for f in *; do
  echo "##### $f"
  grep -n -i "fixinc\|include-fixed\|STMP_FIXINC\|fixincl" "$f" | cut -c1-240 | head -20
done
