"""Slice 9 (plan/headers.md section 4: Linux process and event interfaces): P1 of sys/syscall.h.

SYS_<name> is the number of spec/abi/syscalls.tsv, which is the kernel's __NR_<name>; one unit per configuration.
Loaded by spec/tools/header_check.py, which lists this directory (plan/headers.md section 4, "Adding a slice").
"""


def units(ctx):
    sysrows = ctx.rows("spec/abi/syscalls.tsv")
    body = "#include <sys/syscall.h>\n" + "".join(f"typedef char niji_p1_sys_{r['name']}[(SYS_{r['name']} == {r['number']}) ? 1 : -1];\n" for r in sysrows)
    return [(cid, "P1 sys/syscall.h", body) for cid in ctx.ids]
