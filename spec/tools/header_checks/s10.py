"""Slice 10 (plan/headers.md section 4: locale, text, matching): what the generic units cannot say for this slice.

* `must_fail`: <regexp.h> is shipped to fail in every configuration, with an #error naming the header (the probe table
  in records/pending/SL-HEADERS-S10-2-1.toml: the reference fails the same way, so a configure check reaches the same
  answer). header_check.py leaves it out of P5; the unit below includes it and the runner requires a failure that is
  that #error ("is not provided"), not a missing file or a syntax slip.
* `gettext_units`: the form of <libintl.h>'s __GNU_GETTEXT_SUPPORTED_REVISION (gettext.m4's libc test).
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
    # Spec clause: nl_langinfo(3), the locale-name item (NL-LOCALE-NAME-ABI: perl's perl.h tests `_NL_LOCALE_NAME`, and
    # its interpreter layout follows). Probed form: _NL_ITEM(category, 0xffff), argument and result parenthesised, the
    # type follows the argument.
    forms += [("_NL_LOCALE_NAME(0)", 65535, "int"), ("_NL_LOCALE_NAME(12)", 851967, "int"), ("_NL_LOCALE_NAME(1L)", 131071, "long"),
              ("_NL_LOCALE_NAME(1u)", 131071, "unsigned"), ("_NL_LOCALE_NAME(1) * 2", 262142, "int"),
              ("_NL_LOCALE_NAME(2 - 1)", 131071, "int"), ("_NL_LOCALE_NAME(1 ? 3 : 4)", 262143, "int"),
              ("-_NL_LOCALE_NAME(1)", -131071, "int"), ("_NL_ITEM_INDEX(_NL_LOCALE_NAME(5))", 65535, "int"),
              ("_NL_ITEM_CATEGORY(_NL_LOCALE_NAME(2))", 2, "int")]
    cxx = {c["id"]: c["language"] == "c++" for c in ctx.cfgs}
    for cid in ("c89", "gnu17", "cxx98", "cxx17"):
        same = "__is_same(__typeof__({e}), {t})" if cxx[cid] else "__builtin_types_compatible_p(__typeof__({e}), {t})"
        body = "#include <langinfo.h>\n#if _NL_ITEM(9, 3) != 589827\n#error \"_NL_ITEM in #if\"\n#endif\n"
        body += "".join(f"typedef char niji_nl_{i}[(({e}) == {v} && {same.format(e=e, t=t)}) ? 1 : -1];\n"
                        for i, (e, v, t) in enumerate(forms))
        out.append((cid, "P1 langinfo.h _NL_ITEM forms", body))
    return out + errcode_units(ctx) + gettext_units(ctx) + locale_struct_units(ctx)


# (expression, value, type) of __GNU_GETTEXT_SUPPORTED_REVISION, from black-box probes of the reference
# (HEADERS-GETTEXT-CLANG): 1 for major revision 0, -1 for any other; the argument parenthesised; an int whatever the
# argument's type (an unsigned argument gives a negative result, not a large unsigned one).
GETTEXT_FORMS = [("__GNU_GETTEXT_SUPPORTED_REVISION(0)", 1, "int"), ("__GNU_GETTEXT_SUPPORTED_REVISION(1 - 1)", 1, "int"),
                 ("__GNU_GETTEXT_SUPPORTED_REVISION(0 ? 1 : 0)", 1, "int"), ("__GNU_GETTEXT_SUPPORTED_REVISION(-1)", -1, "int"),
                 ("__GNU_GETTEXT_SUPPORTED_REVISION(1)", -1, "int"), ("__GNU_GETTEXT_SUPPORTED_REVISION(2)", -1, "int"),
                 ("__GNU_GETTEXT_SUPPORTED_REVISION(100000)", -1, "int"), ("__GNU_GETTEXT_SUPPORTED_REVISION(0L)", 1, "int"),
                 ("__GNU_GETTEXT_SUPPORTED_REVISION(1u)", -1, "int"),
                 ("__GNU_GETTEXT_SUPPORTED_REVISION(0) * 2", 2, "int")]


def gettext_units(ctx):
    """Spec clause: gettext.m4's AM_GNU_GETTEXT "GNU gettext in libc" test (gettext's own m4 file; no man page names the
    macro): in each configuration where visibility.tsv shows __GNU_GETTEXT_SUPPORTED_REVISION in <libintl.h>, it is
    usable in #if exactly as that test uses it (`__GNU_GETTEXT_SUPPORTED_REVISION(0) >= 1`), and each GETTEXT_FORMS
    expression has its value and type; where the header is usable but does not show it, it is not defined. Defects
    caught: the macro missing (gettext's test then builds every package without translations), an unparenthesised
    argument, a value claiming a revision for another major, an unsigned or long result."""
    shown = {cid for r in ctx.vis if r["header"] == "libintl.h" and r["name"] == "__GNU_GETTEXT_SUPPORTED_REVISION"
             for cid in header_check.expand(r["configs"], ctx.ids)}
    usable = {cid for r in ctx.vis if r["header"] == "libintl.h" and r["kind"] != "include-error"
              for cid in header_check.expand(r["configs"], ctx.ids)}
    if not shown:
        return [(ctx.ids[0], "libintl.h __GNU_GETTEXT_SUPPORTED_REVISION", "#error visibility.tsv does not show it\n")]
    cxx = {c["id"]: c["language"] == "c++" for c in ctx.cfgs}
    out = []
    for cid in ctx.ids:
        if cid not in shown:
            if cid in usable:
                out.append((cid, "libintl.h __GNU_GETTEXT_SUPPORTED_REVISION (not shown)", "#include <libintl.h>\n"
                            "#ifdef __GNU_GETTEXT_SUPPORTED_REVISION\n#error defined where visibility.tsv does not show it\n#endif\n"))
            continue
        same = "__is_same(__typeof__({e}), {t})" if cxx[cid] else "__builtin_types_compatible_p(__typeof__({e}), {t})"
        body = ("#include <libintl.h>\n#if !(__GNU_GETTEXT_SUPPORTED_REVISION(0) >= 1) || __GNU_GETTEXT_SUPPORTED_REVISION(1) != -1\n"
                "#error \"__GNU_GETTEXT_SUPPORTED_REVISION in #if\"\n#endif\n")
        body += "".join(f"typedef char niji_gt_{i}[(({e}) == {v} && {same.format(e=e, t=t)}) ? 1 : -1];\n"
                        for i, (e, v, t) in enumerate(GETTEXT_FORMS))
        out.append((cid, "libintl.h __GNU_GETTEXT_SUPPORTED_REVISION forms", body))
    return out


# The configurations locale_struct_units samples: the lowest POSIX level with locale_t, its X/Open twin, the GNU
# default, and one C++ mode (the struct's visibility differs per header only by the language and the feature level).
LOCALE_STRUCT_CONFIGS = ("posix200809", "xopen700", "gnu17", "cxx17")
# Headers left out, with the reason: complex.h shows the structure only in C++ modes, where the reference's <complex.h>
# reaches locale_t through the C++ library's headers; libniji's complex.h includes neither, so it shows no locale_t
# there either (an include difference of complex.h, not of this structure).
LOCALE_STRUCT_SKIP = ("complex.h",)


def locale_struct_units(ctx):
    """Spec clause: Appendix C layouts, `struct __locale_struct` (NL-LOCALE-NAME-ABI-3: gnulib's localename module,
    copied into findutils, gettext, gnutls28, grep, libunistring, m4 and sed, reads `locale->__names[category]`, and
    each of those failed to build in Yohane's stage 2 while the tag was incomplete). In every header visibility.tsv
    shows the structure in, for the sampled configurations where it shows it: the structure is complete, with the size,
    alignment and member offsets of layouts.tsv, and `locale_t` points to it (headers/reserved.tsv `__locale_struct`).
    Defects it catches: the tag left incomplete (the reserved row dropped), a member missing or moved, or the
    structure emitted only in some of the headers that show `locale_t`."""
    lay = [r for r in ctx.rows("spec/abi/layouts.tsv") if r["type"] == "struct __locale_struct"]
    whole = next((r for r in lay if not r["member"]), None)
    rows = [r for r in ctx.vis if r["name"] == "__locale_struct" and r["kind"] == "struct" and r["header"] not in LOCALE_STRUCT_SKIP]
    if whole is None or not rows:
        return [(ctx.ids[0], "locale_struct", "#error layouts.tsv or visibility.tsv has no struct __locale_struct row\n")]
    asserts = [f"sizeof (struct __locale_struct) == {whole['offset_or_size']}",
               f"__alignof__ (struct __locale_struct) == {whole['align']}"]
    asserts += [f"__builtin_offsetof (struct __locale_struct, {r['member']}) == {r['offset_or_size']}" for r in lay if r["member"]]
    out = []
    for r in rows:
        shown = header_check.expand(r["configs"], ctx.ids)
        for cid in LOCALE_STRUCT_CONFIGS:
            if cid not in shown:
                continue
            body = f"#include <{r['header']}>\n" + "".join(f"typedef char niji_ls_{i}[({a}) ? 1 : -1];\n" for i, a in enumerate(asserts))
            out.append((cid, f"locale_struct {r['header']}", body))
    return out


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
