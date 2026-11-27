#!/usr/bin/env python3
"""CONFIGURE-DIFF: show why one probe answered differently, from the config.log blocks of both sides.

Usage: why.py <run dir> <pkg> <regex> [<side> ...] [--lines N]
Prints, per side (default stockA and niji), every config.log block ("configure:NNN: checking ...") whose checking
line matches <regex>: the compile command, the compiler's messages and, when the probe failed, the test program
autoconf logged. Meson builds: the matching meson-log.txt "Command line"/"Code" blocks.
"""
import re
import sys
import tarfile


def blocks(text, start):
    cur = []
    for line in text.splitlines():
        if start.match(line) and cur:
            yield cur
            cur = []
        cur.append(line)
    if cur:
        yield cur


def main():
    args = [a for a in sys.argv[1:]]
    n = 60
    if "--lines" in args:
        i = args.index("--lines")
        n = int(args[i + 1])
        del args[i:i + 2]
    run, pkg, pat = args[:3]
    sides = args[3:] or ["stockA", "niji"]
    rx = re.compile(pat)
    for s in sides:
        with tarfile.open("%s/pkgs/%s/out/%s.tar" % (run, pkg, s)) as tf:
            for ti in tf.getmembers():
                base = ti.name.rsplit("/", 1)[-1]
                if base not in ("config.log", "meson-log.txt", "CMakeConfigureLog.yaml"):
                    continue
                text = tf.extractfile(ti).read().decode("utf-8", "replace")
                start = re.compile(r"^configure:\d+: checking |^Running compile:|^Checking ")
                for b in blocks(text, start):
                    if rx.search(b[0]) or (base == "meson-log.txt" and any(rx.search(x) for x in b[:12])):
                        print("==== %s %s %s" % (s, ti.name, b[0][:160]))
                        body = [x for x in b[1:] if not x.startswith(("| #define ", "| /* confdefs.h */"))]
                        for x in body[:n]:
                            print("   " + x[:220])


if __name__ == "__main__":
    main()
