#!/usr/bin/env python3
"""Print the definition of named fixes from gcc's fixincludes/inclhack.def, with every
test_text field removed (test_text quotes header fragments of other libraries).
Usage: fixfields.py <inclhack.def> <fix name>..."""
import re
import sys

path, names = sys.argv[1], set(sys.argv[2:])
text = open(path, encoding="latin-1").read()
lines = text.split("\n")
i = 0
while i < len(lines):
    m = re.match(r"^fix\s*=\s*\{", lines[i])
    if not m:
        i += 1
        continue
    start = i
    # a fix block ends at the first column-0 "};" (braces inside sed strings are unbalanced)
    j = i + 1
    while j < len(lines) and not re.match(r"^\};", lines[j]):
        j += 1
    block = lines[start:j + 1]
    hm = None
    for b in block:
        hm = re.match(r"^\s*hackname\s*=\s*(\w+)\s*;", b)
        if hm:
            break
    if hm and hm.group(1) in names:
        print(f"### {hm.group(1)}  ({path}:{start + 1}-{j + 1})")
        k = 0
        while k < len(block):
            b = block[k]
            if re.match(r"^\s*test_text\s*=", b):
                n0 = k
                hd = re.search(r"<<-?\s*(\w+)", b)
                if hd:
                    # autogen here-string: runs to the terminator line
                    k += 1
                    while k < len(block) and not re.match(r"^\s*" + hd.group(1) + r"\s*;?\s*$", block[k]):
                        k += 1
                else:
                    # a quoted value runs to the line that closes it with ';'
                    while k < len(block) and not re.search(r'"\s*;\s*$', block[k]):
                        k += 1
                print(f"    [test_text omitted: lines {start + 1 + n0}-{start + 1 + k}]")
                k += 1
                continue
            print(f"{start + 1 + k:6d}: {b}")
            k += 1
        print()
    i = j + 1
