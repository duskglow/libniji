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
    out += dl_requests(ctx)
    out += elf_info(ctx)
    levels ={r["id"]: r for r in ctx.rows("headers/levels.tsv")}
    out += [(c["id"], f"P5 {h} must fail", f"#include <{h}>\n") for c in ctx.cfgs if levels[c["id"]]["reject"] != "1" for h in must_fail]
    return out


def dl_requests(ctx):
    """dlinfo(3) and dladdr1(3) (spec 27.1 s27_1_c_6, s27_1_c_7; HEADERS-STAGE2-GAPS-B): where <dlfcn.h> or <link.h>
    shows them, the RTLD_DI_* requests and RTLD_DL_* flags are enumerators of an unnamed enumeration (not macros: a
    use survives `#undef`) with enumerators.tsv's values, and Dl_serinfo's entries are Dl_serpath."""
    vals = {r["name"]: r["value"] for r in ctx.rows("spec/abi/enumerators.tsv") if r["name"].startswith(("RTLD_DI_", "RTLD_DL_"))}
    out = []
    for h in ("dlfcn.h", "link.h"):
        shown = {}
        for r in ctx.vis:
            if r["header"] == h and r["kind"] == "enumerator" and r["name"] in vals:
                for cid in hc.expand(r["configs"], ctx.ids):
                    shown.setdefault(cid, []).append(r["name"])
        for cid, names in sorted(shown.items()):
            body = [f"#include <{h}>"] + [f"#undef {n}" for n in sorted(names)]
            body += [f"typedef char niji_di_{i}[({n}) == {vals[n]} ? 1 : -1];" for i, n in enumerate(sorted(names))]
            body += ["typedef char niji_di_ser[sizeof (((Dl_serinfo *) 0)->dls_serpath[0]) == sizeof (Dl_serpath) ? 1 : -1];"]
            out.append((cid, f"S16 dlinfo requests {h}", "\n".join(body) + "\n"))
    return out


# (use, C type of the result, value): black-box probes of the reference's <elf.h> (HEADERS-STAGE2-GAPS-B leg 4,
# coordinator note for gnu-efi): which operand types reach the result and which bits survive.
ELF_INFO = [
    ("ELF32_R_SYM ((Elf32_Word) 0x12345678)", "unsigned int", "0x123456"), ("ELF32_R_SYM (-16)", "int", "-1"),
    ("ELF32_R_TYPE (0x12345678)", "int", "0x78"), ("ELF32_R_TYPE (-16)", "int", "0xf0"),
    ("ELF32_R_TYPE (2ULL)", "unsigned long long", "2"), ("ELF32_R_INFO (0x123, 0x1ff)", "int", "0x123ff"),
    ("ELF32_R_INFO ((Elf32_Word) 0x123, (Elf32_Word) 0x1ff)", "unsigned int", "0x123ff"),
    ("ELF32_R_INFO (1, 2ULL)", "unsigned long long", "0x102"),
    ("ELF64_R_SYM ((Elf64_Xword) 0x123456789abcdef0ULL)", "unsigned long", "0x12345678"),
    ("ELF64_R_SYM (0x123456789L)", "long", "1"),
    ("ELF64_R_TYPE ((Elf64_Xword) 0x123456789abcdef0ULL)", "unsigned long", "0x9abcdef0"),
    ("ELF64_R_TYPE (-16)", "unsigned int", "0xfffffff0"), ("ELF64_R_INFO (1, 2)", "unsigned long", "0x100000002"),
    ("ELF64_R_INFO (1, 2ULL)", "unsigned long long", "0x100000002"),
    ("ELF64_R_INFO ((Elf64_Xword) 0x89abcdef, (Elf64_Xword) 0x1ffffffff)", "unsigned long", "0x89abcdf0ffffffff"),
    ("ELF64_R_INFO ((Elf64_Xword) 0x123456789abcdef0ULL, -16)", "unsigned long", "0x9abcdeeffffffff0"),
    *[e for w in (32, 64) for e in (
        (f"ELF{w}_ST_BIND (0x1234)", "int", "3"), (f"ELF{w}_ST_BIND (-16)", "int", "0xf"),
        (f"ELF{w}_ST_BIND (2ULL)", "int", "0"), (f"ELF{w}_ST_TYPE (0x1234)", "int", "4"),
        (f"ELF{w}_ST_TYPE (-16)", "int", "0"), (f"ELF{w}_ST_TYPE (2ULL)", "unsigned long long", "2"),
        (f"ELF{w}_ST_INFO (0x1f, 0x2e)", "int", "0x1fe"), (f"ELF{w}_ST_INFO (1, 2ULL)", "unsigned long long", "0x12"),
        (f"ELF{w}_ST_VISIBILITY (0xa7)", "int", "3"), (f"ELF{w}_ST_VISIBILITY (-16)", "int", "0"),
        (f"ELF{w}_ST_VISIBILITY (2ULL)", "unsigned long long", "2"), (f"ELF{w}_M_SYM (-16)", "int", "-1"),
        (f"ELF{w}_M_SIZE (0x1234L)", "unsigned char", "0x34"), (f"ELF{w}_M_SIZE (-16)", "unsigned char", "0xf0"),
        (f"ELF{w}_M_INFO (0x123, 0x1ff)", "int", "0x123ff"), (f"ELF{w}_M_INFO (1ULL, 2)", "unsigned long long", "0x102"))],
    ("ELF32_M_SYM ((Elf32_Word) 0x12345678)", "unsigned int", "0x123456"), ("ELF64_M_SYM (0x1234L)", "long", "0x12"),
    ("ELF32_M_INFO (1, 2ULL)", "int", "0x102"),
]


def elf_info(ctx):
    """ELF gABI "Relocation Entries" and "Symbol Table" (r_info, st_info, st_other) and the move entries' m_info
    (no spec clause beyond the gABI; HEADERS-STAGE2-GAPS-B, gnu-efi): where <elf.h> shows the ELF32_/ELF64_ R_, ST_
    and M_ accessors, each use in ELF_INFO is a constant expression of the recorded type and value, C and C++."""
    names = {e.split(" ")[0] for e, _, _ in ELF_INFO}
    shown = {}
    for r in ctx.vis:
        if r["header"] == "elf.h" and r["kind"] == "macro" and r["name"] in names:
            for cid in hc.expand(r["configs"], ctx.ids):
                shown.setdefault(cid, set()).add(r["name"])
    out = []
    for cid, got in sorted(shown.items()):
        if got != names:
            out.append((cid, "S16 elf.h info macros", f"#error visibility.tsv shows only part of {sorted(names)}\n"))
            continue
        body = ["#include <elf.h>"]
        if cid.startswith("cxx"):
            body += ["template <class A, class B> struct niji_same { enum { v = 0 }; };",
                     "template <class A> struct niji_same<A, A> { enum { v = 1 }; };"]
        for i, (e, t, v) in enumerate(ELF_INFO):
            same = f"niji_same<__typeof__ ({e}), {t}>::v" if cid.startswith("cxx") else \
                f"__builtin_types_compatible_p (__typeof__ ({e}), {t})"
            body.append(f"typedef char niji_ei_{i}[{same} && ({e}) == ({t}) ({v}) ? 1 : -1];")
        out.append((cid, "S16 elf.h info macros", "\n".join(body) + "\n"))
    return out


def types_ns(ctx):
    """The context of P1 restricted to <sys/profil.h> and the names PENDING_INCLUDE lists."""
    ns = type(ctx)(**vars(ctx))
    ns.slice_rows = [{"header": "sys/profil.h", "p1": "1", "uapi": "-"}]
    ns.vis = [r for r in ctx.vis if r["header"] == "sys/profil.h" and (r["kind"] not in ("typedef", "struct") or r["name"] in PENDING_INCLUDE["sys/profil.h"])]
    return ns
