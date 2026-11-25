"""Slice 10 (plan/headers.md section 4: locale, text, matching): what the generic units cannot say for this slice.

* `must_fail`: <regexp.h> is shipped to fail in every configuration, with an #error naming the header (the probe table
  in records/pending/SL-HEADERS-S10-2-1.toml: the reference fails the same way, so a configure check reaches the same
  answer). header_check.py leaves it out of P5; the unit below includes it and the runner requires a failure that is
  that #error ("is not provided"), not a missing file or a syntax slip.
* `p1_skip`: rows of visibility.tsv whose P1 unit would assert what the header does not show.
  - `ftw.h` `statx`, `statx_timestamp`: defined by <sys/stat.h> (slice 7), which ftw.h includes once that header
    exists; drop the entries then.
Loaded by spec/tools/header_check.py (plan/headers.md section 4, "Adding a slice").
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import header_check  # noqa: E402

must_fail = ["regexp.h"]
p1_skip = [("ftw.h", "statx"), ("ftw.h", "statx_timestamp")]


def units(ctx):
    levels = {r["id"]: r for r in ctx.rows("headers/levels.tsv")}
    out = [(c["id"], f"P5 {h} must fail", f"#include <{h}>\n")
           for c in ctx.cfgs if levels[c["id"]]["reject"] != "1" for h in must_fail]
    # Spec clause: regex(3), the GNU interface (HEADERS-SNAPSHOT-GAPS): RE_TRANSLATE_TYPE is shown exactly where the
    # reference shows it and names the type of the pattern buffer's `translate` member (layouts.tsv), which busybox's
    # grep declares its table with.
    member = next(r["member_type"].strip() for r in ctx.rows("spec/abi/layouts.tsv")
                  if r["type"] == "regex_t" and r["member"] == "translate")
    for h in ("regex.h", "re_comp.h"):
        out += ctx.exact(h, ["RE_TRANSLATE_TYPE"], {"RE_TRANSLATE_TYPE": member})
    # Spec clause: nl_langinfo(3), the item numbers (HEADERS-STAGE2-GAPS-A: gnutls's tests use _NL_ITEM, perl the
    # items of the locale categories). The form of the three item macros, from black-box probes of the reference:
    # (expression, value, type); argument and result parenthesised, the category and index macros convert their
    # argument to int first. _NL_ITEM is also usable in #if.
    forms = [("_NL_ITEM(1, 2)", 65538, "int"), ("_NL_ITEM(2 - 1, 0)", 65536, "int"), ("_NL_ITEM(1, 1 ? 2 : 3)", 65538, "int"),
             ("_NL_ITEM(1, 2) * 2", 131076, "int"), ("_NL_ITEM(1L, 2)", 65538, "long"), ("_NL_ITEM(1u, 2)", 65538, "unsigned"),
             ("_NL_ITEM_CATEGORY(0x20005)", 2, "int"), ("_NL_ITEM_CATEGORY(0x10000 | 0x20000)", 3, "int"),
             ("_NL_ITEM_CATEGORY(0xFFFFFFFFu)", -1, "int"), ("_NL_ITEM_CATEGORY(0x100020005L)", 2, "int"),
             ("_NL_ITEM_CATEGORY(0x20005) * 2", 4, "int"), ("_NL_ITEM_INDEX(0x20000 | 5)", 5, "int"),
             ("_NL_ITEM_INDEX(0xFFFFFFFFu)", 65535, "int"), ("_NL_ITEM_INDEX(0x100020005L)", 5, "int"),
             ("_NL_ITEM_INDEX(0x20005) * 2", 10, "int"), ("_NL_ITEM_INDEX(_NL_ADDRESS_COUNTRY_AB2)", 3, "int"),
             ("_NL_ITEM_CATEGORY(_NL_IDENTIFICATION_CODESET)", 12, "int")]
    cxx = {c["id"]: c["language"] == "c++" for c in ctx.cfgs}
    for cid in ("c89", "gnu17", "cxx98", "cxx17"):
        same = "__is_same(__typeof__({e}), {t})" if cxx[cid] else "__builtin_types_compatible_p(__typeof__({e}), {t})"
        body = "#include <langinfo.h>\n#if _NL_ITEM(9, 3) != 589827\n#error \"_NL_ITEM in #if\"\n#endif\n"
        body += "".join(f"typedef char niji_nl_{i}[(({e}) == {v} && {same.format(e=e, t=t)}) ? 1 : -1];\n"
                        for i, (e, v, t) in enumerate(forms))
        out.append((cid, "P1 langinfo.h _NL_ITEM forms", body))
    # Spec clause: ftw(3) (FTW_F and friends report a stat structure) and the header's includes by black-box probe
    # (HEADERS-STAGE2-GAPS-E: libcap-ng's filecap.c tests S_ISREG after <ftw.h> alone): the S_IS* tests exactly where
    # the reference shows them through <ftw.h>.
    out += ctx.exact("ftw.h", FTW_STAT_TESTS)
    return out + errcode_units(ctx)


FTW_STAT_TESTS = ("S_ISBLK", "S_ISCHR", "S_ISDIR", "S_ISFIFO", "S_ISREG", "S_ISLNK", "S_ISSOCK")


def errcode_units(ctx):
    """Spec clause: regex(3) error codes (the REG_* values regcomp and regexec return); their type by black-box probe
    (HEADERS-ENUM-FOLLOWUPS): per configuration that shows it, <regex.h> declares `reg_errcode_t`, an int-sized
    enumeration, and each REG_* error code of enum-macros.tsv (a macro naming its `_REG_*` enumerator) converts to it
    without a cast in C++ and has the enumerators.tsv value. Defects it catches: no `reg_errcode_t` (an int-only
    REG_NOMATCH fails `reg_errcode_t e = REG_NOMATCH;` in C++), a structure in its place, a wrong or missing value."""
    # The error codes only (<sys/ucontext.h>'s REG_* register names are enum-macros.tsv rows too).
    codes = {r["name"]: r["enumerator"] for r in ctx.rows("spec/abi/enum-macros.tsv") if r["name"].startswith("REG_") and r["enumerator"] == "_" + r["name"]}
    value = {r["name"]: r["value"] for r in ctx.rows("spec/abi/enumerators.tsv") if r["tag"] == "typedef reg_errcode_t"}
    td = next((r for r in ctx.vis if r["header"] == "regex.h" and r["name"] == "reg_errcode_t" and r["kind"] == "typedef"), None)
    if td is None or len(codes) < 18 or any(e not in value for e in codes.values()):
        return [(ctx.ids[0], "reg_errcode_t regex.h", "#error the tables do not show reg_errcode_t and its 18 REG_* codes\n")]
    lang = {c["id"]: c["language"] for c in ctx.cfgs}
    shown = {r["name"]: r["configs"] for r in ctx.vis if r["header"] == "regex.h" and r["kind"] == "macro" and r["name"] in codes}
    res = []
    for cid in sorted(header_check.expand(td["configs"], ctx.ids)):
        body = ["#include <regex.h>", "typedef char niji_re_sz[sizeof (reg_errcode_t) == 4 ? 1 : -1];"]
        if lang[cid] == "c++":
            body.append("typedef char niji_re_enum[__is_enum (reg_errcode_t) ? 1 : -1];")
        for i, (n, e) in enumerate(sorted(codes.items())):
            if cid not in header_check.expand(shown.get(n, ""), ctx.ids):
                continue  # REG_ENOSYS is shown in some configurations only
            body += [f"reg_errcode_t niji_re{i} = {n};", f"typedef char niji_rv{i}[({n}) == ({value[e]}) ? 1 : -1];"]
        body.append("int niji_re_f (reg_errcode_t e) { switch (e) { case REG_NOERROR: return 0; case REG_NOMATCH: return 1; default: return 2; } }")
        res.append((cid, "reg_errcode_t regex.h", "\n".join(body) + "\n"))
    return res
