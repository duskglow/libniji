"""Enumeration probe (HEADERS-CXX-ENUM-TYPES): which macros and enumerator rows of each header are enumerators of an
enumeration in the reference, and which enumeration (black box: a program compiled against the build host's C library
headers is run and prints what the language says of each name; no header text is read).

Per (header, name) the C++17 unit (`-D_GNU_SOURCE`, the widest visibility) prints: whether `decltype((NAME))` is an
enumeration (`__is_enum`); the enumeration's name as the compiler spells the type (`<unnamed enum>` for one without
a name or typedef); its group (the first name of the header, in table order, whose type is the same type:
`std::type_index`); its underlying type (`__underlying_type`) and `sizeof`; the value. The C unit (gnu17) prints
whether the C preprocessor evaluates the macro as 0 where its value is not 0 (the macro expands to an identifier,
an enumerator, as in C++).

Output (--out): header name is_enum enum group underlying sizeof value c_pp_identifier

Usage: python3 spec/tools/enum_probe.py --out FILE [--headers H,...] [--work DIR] [--jobs N]
"""
import argparse
import concurrent.futures
import os
import re
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "header_checks"))
import form  # noqa: E402

ROOT = form.ROOT
PRELUDE = r"""#include <cstdio>
#include <map>
#include <string>
#include <typeindex>
#include <typeinfo>
template<class T> const char *niji_nm() { return __PRETTY_FUNCTION__; }
template<class T, bool E = __is_enum(T)> struct niji_u { static const char *n() { return "-"; } };
template<class T> struct niji_u<T, true> { static const char *n() { return niji_nm<__underlying_type(T)>(); } };
static std::map<std::type_index, std::string> niji_g;
template<class T> void niji_rec(const char *n, T v) {
  auto it = niji_g.emplace(std::type_index(typeid(T)), n).first;
  std::printf("%s\t%d\t%s\t%s\t%s\t%zu\t%s\n", n, (int) __is_enum(T), niji_nm<T>(), it->second.c_str(), niji_u<T>::n(),
              sizeof(T), (v < 0 ? std::to_string((long long) v) : std::to_string((unsigned long long) v)).c_str());
}
int main() {
"""
WITH = re.compile(r"\[with T = (.*)\]$")


def names_by_header():
    """header -> names (macro rows of integer macros.tsv types and enumerator rows), in table order."""
    macros = {}
    with open(os.path.join(ROOT, "spec/abi/macros.tsv")) as f:
        head = f.readline().rstrip("\n").split("\t")
        for line in f:
            r = dict(zip(head, line.rstrip("\n").split("\t")))
            macros[r["name"]] = r["type"]
    out = {}
    with open(os.path.join(ROOT, "spec/abi/visibility.tsv")) as f:
        head = f.readline().rstrip("\n").split("\t")
        for line in f:
            r = dict(zip(head, line.rstrip("\n").split("\t")))
            ok = r["kind"] == "enumerator" or (r["kind"] == "macro" and macros.get(r["name"]) in form.INT_TYPES)
            if ok and r["header"] != "-" and r["name"] not in out.setdefault(r["header"], []):
                out[r["header"]].append(r["name"])
    return out


def cxx_unit(header, items):
    lines = [f"#include <{header}>"] + PRELUDE.rstrip("\n").split("\n")
    at = {}
    for i, n in items:
        lines.append(f"niji_rec(\"{n}\", ({n}));")
        at[len(lines)] = (i, n)
    lines.append("}")
    return "\n".join(lines) + "\n", at


def c_unit(header, items):
    lines = [f"#include <{header}>", "#include <stdio.h>", "int main(void) {"]
    at = {}
    for i, n in items:
        lines.append(f"#if !({n})")
        lines.append(f"if (({n}) != 0) puts(\"{n}\\tid\");")
        at[len(lines) - 1] = at[len(lines)] = (i, n)
        lines.append("#else")
        lines.append(f"puts(\"{n}\\tlit\");")
        lines.append("#endif")
    lines.append("}")
    return "\n".join(lines) + "\n", at


def run_unit(cmd, src, exe, text_of, items):
    done, ok = form.compile_dropping(cmd + ["-o", exe], src, text_of, items)
    if not done or not ok:
        return ""
    return subprocess.run([exe], capture_output=True, text=True, timeout=60).stdout


def probe(header, names, work):
    stem = os.path.join(work, header.replace("/", "_"))
    items = list(enumerate(names))
    diag = ["-fdiagnostics-plain-output", "-fno-diagnostics-show-caret", "-w"]
    rows = {}
    out = run_unit(["g++", "-std=gnu++17", "-D_GNU_SOURCE"] + diag, stem + ".cc", stem + "-cc", lambda xs: cxx_unit(header, xs), items)
    for line in out.splitlines():
        n, en, ty, grp, ut, sz, v = line.split("\t")
        m, u = WITH.search(ty), WITH.search(ut)
        rows[n] = [n, en, m.group(1) if m else ty, grp, u.group(1) if u else ut, sz, v, "0"]
    cout = run_unit(["gcc", "-D_GNU_SOURCE"] + diag, stem + ".c", stem + "-c", lambda xs: c_unit(header, xs), items)
    for line in cout.splitlines():
        n, _, how = line.partition("\t")
        if n in rows:
            rows[n][7] = "1" if how == "id" else "lit"
    return [[header] + rows[n] for n in names if n in rows]


def tsv(path):
    with open(path) as f:
        head = f.readline().rstrip("\n").split("\t")
        return [dict(zip(head, line.rstrip("\n").split("\t"))) for line in f]


def derive(probe_tsv):
    """The table rows of a probe run: enumerators.tsv rows `anon FIRST` for each enumeration without a name or typedef
    (FIRST: its first enumerator by name; values as probed, ordered by value), and enum-macros.tsv (each macro that is
    an enumerator in the reference, and the enumerator it names: itself, or the enumerator of the same enumeration
    with its value; a macro with no such enumerator in the tables names a new reserved one, `__niji_NAME`).
    Returns (anonymous rows, macro rows, notes)."""
    par = {}

    def find(x):
        while par.setdefault(x, x) != x:
            par[x] = par[par[x]]
            x = par[x]
        return x
    every = tsv(probe_tsv)
    probe_rows = [r for r in every if r["is_enum"] == "1"]
    # The headers where a macro that is an enumerator elsewhere is not one (IPPORT_RESERVED is an `int` in <netdb.h>).
    not_in = {}
    for r in every:
        if r["is_enum"] == "0":
            not_in.setdefault(r["name"], set()).add(r["header"])
    for r in probe_rows:
        par[find(r["name"])] = find(r["group"])
    groups, facts = {}, {}
    for r in probe_rows:
        groups.setdefault(find(r["name"]), set()).add(r["name"])
        facts.setdefault(r["name"], []).append(r)
    kinds = {}
    for r in tsv(os.path.join(ROOT, "spec/abi/visibility.tsv")):
        kinds.setdefault(r["name"], set()).add(r["kind"])
    etsv = {}
    for r in tsv(os.path.join(ROOT, "spec/abi/enumerators.tsv")):
        if not r["tag"].startswith("anon "):
            etsv.setdefault(r["tag"], []).append((r["name"], r["value"]))
    tag_of = {n: t for t, ms in etsv.items() for n, _ in ms}
    anon, macros, notes = [], [], []
    for g in sorted(groups.values(), key=min):
        val = {n: facts[n][0]["value"] for n in g}
        ename = facts[min(g)][0]["enum"]
        tags = {tag_of[n] for n in g if n in tag_of} | {t for t in etsv if ename != "<unnamed enum>" and t.split(" ", 1)[1] == ename}
        if len(tags) > 1:
            notes.append(f"{min(g)}: more than one tag {sorted(tags)}")
            continue
        tag = tags.pop() if tags else None
        if tag:
            pool = list(etsv[tag])
        else:
            pool = sorted(((n, val[n]) for n in g if "enumerator" in kinds.get(n, ())), key=lambda m: (int(m[1]), m[0]))
        members = {n for n, _ in pool}
        hidden = []
        for n in sorted(g):
            if "macro" not in kinds.get(n, ()):
                continue
            if any(r["c_pp_identifier"] == "lit" for r in facts[n]):
                notes.append(f"{n}: a literal in C, an enumerator in C++ (not derived)")
                continue
            if n in members:
                macros.append((n, n))
                continue
            same = sorted((m for m, v in pool if v == val[n]), key=lambda m: (m.startswith("_"), m))
            if same:
                macros.append((n, same[0]))
            elif tag or not pool:
                notes.append(f"{n}: no enumerator of its enumeration has its value")
            else:
                hidden.append((f"__niji_{n}", val[n]))
                macros.append((n, f"__niji_{n}"))
        if pool and not tag:
            first = min(m for m, _ in pool)
            anon += [(f"anon {first}", m, v) for m, v in sorted(pool + hidden, key=lambda m: (int(m[1]), m[0]))]
    return anon, sorted((n, e, ",".join(sorted(not_in.get(n, ())))) for n, e in macros), notes


def write_tables(probe_tsv):
    anon, macros, notes = derive(probe_tsv)
    path = os.path.join(ROOT, "spec/abi/enumerators.tsv")
    keep = [r for r in tsv(path) if not r["tag"].startswith("anon ")]
    with open(path, "w") as f:
        f.write("tag\tname\tvalue\n")
        for r in keep:
            f.write(f"{r['tag']}\t{r['name']}\t{r['value']}\n")
        for t, n, v in anon:
            f.write(f"{t}\t{n}\t{v}\n")
    with open(os.path.join(ROOT, "spec/abi/enum-macros.tsv"), "w") as f:
        f.write("name\tenumerator\tnot_in\n")
        for n, e, x in macros:
            f.write(f"{n}\t{e}\t{x}\n")
    for x in notes:
        print(f"NOTE {x}")
    print(f"enumerators.tsv: {len(anon)} rows of {len({t for t, _, _ in anon})} enumerations without a name; "
          f"enum-macros.tsv: {len(macros)} macros")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--derive", default="", help="write enumerators.tsv's `anon` rows and enum-macros.tsv from a probe TSV")
    ap.add_argument("--out", default="")
    ap.add_argument("--headers", default="")
    ap.add_argument("--work", default="")
    ap.add_argument("--jobs", type=int, default=min(16, os.cpu_count() or 1))
    a = ap.parse_args()
    if a.derive:
        write_tables(a.derive)
        return 0
    work = a.work or tempfile.mkdtemp(prefix="enum.")
    os.makedirs(work, exist_ok=True)
    by = names_by_header()
    skip = set(form.must_fail_headers())
    only = set(a.headers.split(",")) if a.headers else None
    hs = [h for h in sorted(by) if h not in skip and (only is None or h in only)]
    with concurrent.futures.ThreadPoolExecutor(max_workers=a.jobs) as ex:
        res = list(ex.map(lambda h: probe(h, by[h], work), hs))
    with open(a.out, "w") as f:
        f.write("header\tname\tis_enum\tenum\tgroup\tunderlying\tsizeof\tvalue\tc_pp_identifier\n")
        for rows in res:
            for r in rows:
                f.write("\t".join(r) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
