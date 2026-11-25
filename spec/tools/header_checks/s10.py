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
    return out
