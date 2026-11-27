#!/usr/bin/env python3
"""Make a scratch variant of a libniji header tree in which va_list (stdio.h, wchar.h) and size_t
(stdlib.h, unistd.h) come from the compiler's own <stdarg.h>/<stddef.h> through their __need_* protocol.
Usage: variant.py <source include dir> <dest include dir>   (dest must not exist)"""
import shutil
import sys

src, dst = sys.argv[1], sys.argv[2]
shutil.copytree(src, dst, symlinks=True)

VA_OLD = ("#  ifndef _VA_LIST_DEFINED\n#   define _VA_LIST_DEFINED\n"
          "typedef __builtin_va_list va_list;\n#  endif\n")
VA_NEW = ("#  define __need___va_list\n#  include <stdarg.h>\n"
          "typedef __gnuc_va_list va_list;\n")
SZ_OLD = "#  define __NIJI_D_size_t 1\ntypedef unsigned long size_t;\n"
SZ_NEW = "#  define __NIJI_D_size_t 1\n#  define __need_size_t\n#  include <stddef.h>\n"

WC_OLD = "#  ifndef __cplusplus\ntypedef __WCHAR_TYPE__ wchar_t;\n#  endif\n"
WC_NEW = "#  ifndef __cplusplus\n#   define __need_wchar_t\n#   include <stddef.h>\n#  endif\n"

for name, old, new in (("stdio.h", VA_OLD, VA_NEW), ("wchar.h", VA_OLD, VA_NEW),
                       ("stdlib.h", SZ_OLD, SZ_NEW), ("unistd.h", SZ_OLD, SZ_NEW),
                       ("stdlib.h", WC_OLD, WC_NEW)):
    p = f"{dst}/{name}"
    t = open(p, encoding="utf-8").read()
    n = t.count(old)
    if n != 1:
        sys.exit(f"{name}: expected 1 block, found {n}")
    open(p, "w", encoding="utf-8").write(t.replace(old, new))
    print(f"{name}: replaced 1 block")
