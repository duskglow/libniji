"""Slice 16 (plan/headers.md section 4: ELF, dynamic linking, debugging): the checks only its headers need.

* member offsets: for each structure or typedef the slice's headers make visible, `__builtin_offsetof` of every plain
  member of layouts.tsv (the shared P1 checks size and alignment only);
* `<sys/profil.h>`: the reference's header includes `<sys/time.h>` (slice 7, not generated yet), which brings fd_set,
  sigset_t, the pthread types and more at every level. Until that header exists the shared P1 skips it (its row has
  `p1` 0) and this module runs P1 on the one structure `<sys/profil.h>` itself declares, `struct prof`;
* `<gnu/lib-names.h>`: one string macro per library of versions.tsv, with the library's soname;
* `must_fail`: `<sys/elf.h>` is shipped to fail in every configuration (D-X.130 (3)), with an #error naming the header.
  header_check.py leaves it out of P5; the unit below includes it and the runner requires a failure that is that #error
  ("is not provided"), not a missing file or a syntax slip.
Loaded by spec/tools/header_check.py, which lists this directory.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import header_check as hc  # noqa: E402

PENDING_INCLUDE = {"sys/profil.h": {"prof"}}
must_fail = ["sys/elf.h"]
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
    levels = {r["id"]: r for r in ctx.rows("headers/levels.tsv")}
    out += [(c["id"], f"P5 {h} must fail", f"#include <{h}>\n") for c in ctx.cfgs if levels[c["id"]]["reject"] != "1" for h in must_fail]
    # Spec clause: ar(5) / the System V archive format (HEADERS-STAGE2-GAPS-C, elfutils' libelf): the magic string, its
    # length and the member-header trailer, and `struct ar_hdr` as the format lays it out (fixed-width character fields,
    # 60 bytes), asserted from the format itself rather than from the tables the header is generated from.
    fields = (("ar_name", 0, 16), ("ar_date", 16, 12), ("ar_uid", 28, 6), ("ar_gid", 34, 6), ("ar_mode", 40, 8),
              ("ar_size", 48, 10), ("ar_fmag", 58, 2))
    body = ["#include <ar.h>", "typedef char niji_ar_magic[__builtin_strcmp (ARMAG, \"!<arch>\\n\") == 0 && SARMAG == 8"
            " && sizeof (ARMAG) == 9 ? 1 : -1];",
            "typedef char niji_ar_fmag[__builtin_strcmp (ARFMAG, \"`\\n\") == 0 && sizeof (ARFMAG) == 3 ? 1 : -1];",
            "#if SARMAG != 8\n#error \"SARMAG\"\n#endif",
            "typedef char niji_ar_size[sizeof (struct ar_hdr) == 60 && __alignof__ (struct ar_hdr) == 1 ? 1 : -1];"]
    body += [f"typedef char niji_ar_{m}[__builtin_offsetof (struct ar_hdr, {m}) == {o}"
             f" && sizeof (((struct ar_hdr *) 0)->{m}) == {n} && sizeof (((struct ar_hdr *) 0)->{m}[0]) == 1 ? 1 : -1];"
             for m, o, n in fields]
    out += [(cid, "S16 ar.h format", "\n".join(body) + "\nint niji_ar_tu;\n") for cid in ("c89", "gnu17", "cxx98", "cxx17")]
    return out


def types_ns(ctx):
    """The context of P1 restricted to <sys/profil.h> and the names PENDING_INCLUDE lists."""
    ns = type(ctx)(**vars(ctx))
    ns.slice_rows = [{"header": "sys/profil.h", "p1": "1", "uapi": "-"}]
    ns.vis = [r for r in ctx.vis if r["header"] == "sys/profil.h" and (r["kind"] not in ("typedef", "struct") or r["name"] in PENDING_INCLUDE["sys/profil.h"])]
    return ns
