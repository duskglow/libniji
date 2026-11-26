"""Slice 10 checks: stage2-gaps-g (header_checks/s10.d/, loaded after s10.py by spec/tools/header_check.py;
header_checks/README.md)."""


def units(ctx):
    """No spec clause beyond fts(3) (black-box probe of the reference's layout, HEADERS-STAGE2-GAPS-G: fakechroot's
    fts64.c uses FTS64's fts_compar and fts_options): where visibility.tsv shows FTS64 and FTSENT64 in <fts.h>, every
    member layouts.tsv records for them is at its offset (P1 checks the sizes), fts_ino is ino64_t and fts_statp struct stat64 *
    (the 64-bit spellings where FTSENT has ino_t and struct stat *). Defects: the types left opaque (fakechroot does not
    build), a member moved, the 32-bit-named member types (wrong in C++ overloads and _FILE_OFFSET_BITS=32 code)."""
    lang = {c["id"]: c["language"] for c in ctx.cfgs}
    rows = [r for r in ctx.rows("spec/abi/layouts.tsv") if r["type"] in ("FTS64", "FTSENT64") and r["member"]]
    vis = {r["name"]: r["configs"] for r in ctx.vis if r["header"] == "fts.h" and r["kind"] == "typedef"}
    out = []
    for cid in ctx.ids:
        if "FTSENT64" not in vis or cid not in ctx.expand(vis["FTSENT64"]):
            continue
        cxx = lang[cid] == "c++"

        def same(a, b):
            return f"__is_same ({a}, {b})" if cxx else f"__builtin_types_compatible_p ({a}, {b})"

        body = ["#include <sys/types.h>", "#include <sys/stat.h>", "#include <fts.h>", "#include <stddef.h>"]
        for i, r in enumerate(rows):
            body.append(f"typedef char niji_fo{i}[offsetof ({r['type']}, {r['member']}) == {r['offset_or_size']} ? 1 : -1];")
        body.append(f"typedef char niji_ft1[{same('__typeof__ (((FTSENT64 *) 0)->fts_ino)', 'ino64_t')} ? 1 : -1];")
        body.append(f"typedef char niji_ft2[{same('__typeof__ (((FTSENT64 *) 0)->fts_statp)', 'struct stat64 *')} ? 1 : -1];")
        body.append(f"typedef char niji_ft3[{same('__typeof__ (((FTS64 *) 0)->fts_cur)', 'FTSENT64 *')} ? 1 : -1];")
        out.append((cid, "FTS64 and FTSENT64 members", "\n".join(body) + "\n"))
    return out
