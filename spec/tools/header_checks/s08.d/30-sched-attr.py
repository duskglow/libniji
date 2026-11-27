"""Slice 8 checks: sched-attr (header_checks/s08.d/, loaded after s08.py by spec/tools/header_check.py;
header_checks/README.md)."""
from header_check import expand

# struct sched_attr (the kernel UAPI's, <linux/sched/types.h>): member -> (offset, type). Black-box probe of the 2.41
# interface (HEADERS-STAGE2-GAPS-H): sizeof 56, alignment 8.
MEMBERS = {"size": (0, "unsigned int"), "sched_policy": (4, "unsigned int"), "sched_flags": (8, "unsigned long long"),
           "sched_nice": (16, "int"), "sched_priority": (20, "unsigned int"), "sched_runtime": (24, "unsigned long long"),
           "sched_deadline": (32, "unsigned long long"), "sched_period": (40, "unsigned long long"),
           "sched_util_min": (48, "unsigned int"), "sched_util_max": (52, "unsigned int")}
VALUES = {"SCHED_NORMAL": 0, "SCHED_EXT": 7, "SCHED_FLAG_RESET_ON_FORK": 1, "SCHED_FLAG_RECLAIM": 2,
          "SCHED_FLAG_DL_OVERRUN": 4, "SCHED_FLAG_KEEP_POLICY": 8, "SCHED_FLAG_KEEP_PARAMS": 16,
          "SCHED_FLAG_UTIL_CLAMP_MIN": 32, "SCHED_FLAG_UTIL_CLAMP_MAX": 64, "SCHED_FLAG_KEEP_ALL": 24,
          "SCHED_FLAG_UTIL_CLAMP": 96, "SCHED_ATTR_SIZE_VER0": 48, "SCHED_ATTR_SIZE_VER1": 56}
PROTOS = {"sched_setattr": "int (__pid_t, struct sched_attr *, unsigned int)",
          "sched_getattr": "int (__pid_t, struct sched_attr *, unsigned int, unsigned int)"}


def units(ctx):
    """Spec section 24.3 clause s24_3_c_6's declarations (owner ruling R3; util-linux's chrt and uclampset use them
    once configure finds sched_setattr): where visibility.tsv shows sched_setaffinity in <sched.h> (the GNU
    configurations), (1) struct sched_attr has the kernel's layout and member types, (2) sched_setattr and
    sched_getattr have their 2.41 types (C++: noexcept), (3) the SCHED_* names have the kernel's values,
    SCHED_FLAG_ALL is not defined, and a program defining SCHED_FLAG_KEEP_ALL the kernel's way after the header gets
    no redefinition warning; elsewhere (4) none of the names is declared or defined. Defects: a struct of its own
    (a redefinition beside <linux/sched/types.h>), a wrong member type or offset (a kernel that reads the wrong
    field), a const or missing parameter, a flag value that differs from the kernel's, the names shown in strict
    modes (a conflict with a program's own)."""
    lang = {c["id"]: c["language"] for c in ctx.cfgs}
    gnu = expand({r["name"]: r["configs"] for r in ctx.vis if r["header"] == "sched.h"}.get("sched_setaffinity", ""), ctx.ids)
    res = []
    for cid in ctx.ids:
        cxx = lang[cid] == "c++"

        def same(a, b):
            return f"__is_same ({a}, {b})" if cxx else f"__builtin_types_compatible_p ({a}, {b})"

        if cid not in gnu:
            body = ["#include <sched.h>", "struct sched_attr { int niji_x; };", "int sched_setattr, sched_getattr;"]
            body += [f"#ifdef {m}\n#error {m} shown\n#endif" for m in VALUES]
            res.append((cid, "sched_attr absent sched.h", "\n".join(body) + "\n"))
            continue
        body = ["#include <stddef.h>", "#include <sched.h>", "struct sched_attr niji_a;",
                "typedef char niji_sz[sizeof (struct sched_attr) == 56 && __alignof__ (struct sched_attr) == 8 ? 1 : -1];"]
        for m, (off, t) in MEMBERS.items():
            body.append(f"typedef char niji_{m}[offsetof (struct sched_attr, {m}) == {off} && {same(f'__typeof__ (niji_a.{m})', t)} ? 1 : -1];")
        for f, t in PROTOS.items():
            body.append(f"typedef char niji_{f}[{same(f'__typeof__ ({f})', t + (' noexcept' if cid in ('cxx17', 'cxx20', 'cxx23') else ''))} ? 1 : -1];")
        body += [f"typedef char niji_{m}[{m} == {v} ? 1 : -1];" for m, v in VALUES.items()]
        body += ["#ifdef SCHED_FLAG_ALL\n#error SCHED_FLAG_ALL shown\n#endif",
                 "#define SCHED_FLAG_KEEP_ALL (SCHED_FLAG_KEEP_POLICY | SCHED_FLAG_KEEP_PARAMS)"]
        res.append((cid, "sched_attr declared sched.h", "\n".join(body) + "\n"))
    return res
