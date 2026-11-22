"""Slice 16 (plan/headers.md section 4: ELF, dynamic linking, debugging): the checks only its headers need.

* member offsets: for each structure or typedef the slice's headers make visible, `__builtin_offsetof` of every plain
  member of layouts.tsv (the shared P1 checks size and alignment only);
* `<sys/profil.h>`: the reference's header includes `<sys/time.h>` (slice 7, not generated yet), which brings fd_set,
  sigset_t, the pthread types and more at every level. Until that header exists the shared P1 skips it (its row has
  `p1` 0) and this module runs P1 on the one structure `<sys/profil.h>` itself declares, `struct prof`;
* `<gnu/lib-names.h>`: one string macro per library of versions.tsv, with the library's soname.
Loaded by spec/tools/header_check.py, which lists this directory.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import header_check as hc  # noqa: E402

PENDING_INCLUDE = {"sys/profil.h": {"prof"}}
SLICE16 = ["dlfcn.h", "link.h", "elf.h", "ar.h", "sys/gmon.h", "sys/gmon_out.h", "sys/profil.h", "gnu/lib-names.h", "sys/platform/x86.h"]


def members(ctx):
    out = {}
    for r in ctx.rows("spec/abi/layouts.tsv"):
        m = r["member"]
        if m and "." not in m and "<" not in m and not r["bitfield"] and "<anon>" not in r["member_type"]:
            out.setdefault(r["type"], []).append((m, r["offset_or_size"]))
    return out


def units(ctx):
    mem = members(ctx)
    out = []
    per = {}
    for r in ctx.vis:
        h = r["header"]
        if h not in SLICE16 or r["kind"] not in ("typedef", "struct"):
            continue
        n = r["name"]
        if n.startswith("__pthread"):
            # The pthread-internal structures reach link.h and sys/gmon.h through <sys/types.h>; the slice that owns that
            # header checks them (layouts.tsv now lists their members), not slice 16.
            continue
        if h in PENDING_INCLUDE and n not in PENDING_INCLUDE[h]:
            continue
        key = n if r["kind"] == "typedef" else "struct " + n
        if r["kind"] == "typedef" and (n not in ctx.typedefs or ctx.typedefs[n]["header"] != h):
            continue
        if not mem.get(key) or (r["kind"] == "struct" and key not in ctx.layouts):
            continue
        tests = [f"__builtin_offsetof({key}, {m}) == {o}" for m, o in mem[key]]
        for cid in hc.expand(r["configs"], ctx.ids):
            per.setdefault((h, cid), []).extend(tests)
    for (h, cid), tests in sorted(per.items()):
        body = f"#include <{h}>\n" + "".join(f"typedef char niji_s16_off_{i}[({t}) ? 1 : -1];\n" for i, t in enumerate(tests))
        out.append((cid, f"S16 offsets {h}", body))
    # sys/profil.h: P1 of what the header itself declares, with the other rows left out.
    sub = types_ns(ctx)
    for cid, units_ in hc.p1_units(sub).items():
        out += [(cid, what, body) for what, body in units_]
    # gnu/lib-names.h: the soname of every library.
    names = sorted({r["library"] for r in ctx.rows("spec/abi/versions.tsv")})
    body = "#include <gnu/lib-names.h>\n" + "".join(
        f"typedef char niji_so_{i}[__builtin_strcmp({n.split('.so')[0].upper().replace('-', '_')}_SO, \"{n}\") == 0 ? 1 : -1];\n"
        for i, n in enumerate(names))
    out += [(cid, "S16 gnu/lib-names.h", body) for cid in ctx.ids]
    return out


def types_ns(ctx):
    """The context of P1 restricted to <sys/profil.h> and the names PENDING_INCLUDE lists."""
    ns = type(ctx)(**vars(ctx))
    ns.slice_rows = [{"header": "sys/profil.h", "p1": "1", "uapi": "-"}]
    ns.vis = [r for r in ctx.vis if r["header"] == "sys/profil.h" and (r["kind"] not in ("typedef", "struct") or r["name"] in PENDING_INCLUDE["sys/profil.h"])]
    return ns
