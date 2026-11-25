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
    return out + errcode_units(ctx)


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
