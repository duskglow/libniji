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
import header_check as hc  # noqa: E402

must_fail = ["regexp.h"]
p1_skip = [("ftw.h", "statx"), ("ftw.h", "statx_timestamp")]


def units(ctx):
    levels = {r["id"]: r for r in ctx.rows("headers/levels.tsv")}
    out = [(c["id"], f"P5 {h} must fail", f"#include <{h}>\n")
           for c in ctx.cfgs if levels[c["id"]]["reject"] != "1" for h in must_fail]
    # Spec clause: gettext(3), ngettext(3) (HEADERS-STAGE2-GAPS-B: avahi's `printf (_("..."))` under
    # -Werror=format-security). Each message argument the reference marks format_arg (black-box attribute probe,
    # diagattrs.tsv `format_arg`) passes a literal's format checking through: a literal through any of them is a
    # literal format, so -Wformat-security stays silent where the call has no other argument.
    calls = {"gettext": '"x"', "dgettext": '"d", "x"', "dcgettext": '"d", "x", 5', "ngettext": '"x", "y", n',
             "dngettext": '"d", "x", "y", n', "dcngettext": '"d", "x", "y", n, 5'}
    fa = {}
    for r in ctx.vis:
        if r["header"] == "libintl.h" and r["kind"] == "function" and r["name"] in calls:
            for cid in hc.expand(r["configs"], ctx.ids):
                fa.setdefault(cid, []).append(r["name"])
    for cid, fns in sorted(fa.items()):
        body = ["#include <libintl.h>", '#pragma GCC diagnostic error "-Wformat"', '#pragma GCC diagnostic error "-Wformat-security"',
                "extern int niji_pf (const char *, ...) __attribute__ ((__format__ (__printf__, 1, 2)));",
                "void niji_fa (unsigned long n) { (void) n;"] + [f"  niji_pf ({f} ({calls[f]}));" for f in sorted(fns)] + ["}"]
        out.append((cid, "format_arg libintl.h", "\n".join(body) + "\n"))
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
    return out
