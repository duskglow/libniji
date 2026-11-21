#!/bin/sh
# LAYOUT-TOOL-BOUND-SHARED: setup.sh - a bare RAM mirror of next and the three branches build.sh merges.
set -eu
R=/mnt/zram/claude/LAYOUT-TOOL-BOUND-SHARED
mkdir -p "$R/out"
git init -q --bare "$R/mirror.git"
git -C "$R/mirror.git" fetch -q https://github.com/duskglow/liella.git \
    next:refs/heads/next \
    task/tb-foreign-words-low-10:refs/heads/t10 \
    task/tb-foreign-words-low-8:refs/heads/t8 \
    task/setlocale-ctype-env-4:refs/heads/sce4 \
    task/regcomp-fallible-alloc-2:refs/heads/rfa2
git -C "$R/mirror.git" for-each-ref --format='%(refname:short) %(objectname:short=9)'
