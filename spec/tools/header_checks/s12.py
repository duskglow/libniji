"""Slice 12 (plan/headers.md section 4: network core and name services): P1 of its sixteen headers with member offsets,
and P10, the network macros' output against the build host's own headers.

* P1  (`units(ctx)`): per (header, configuration) one unit that includes the header and asserts, for what it makes visible
  there, every macros.tsv integer value, every typedef and structure size and alignment, and the offset of every
  structure member that is not a bit-field. Names another slice's header owns are left to that header's P1 (signal.h and
  sys/param.h belong to slices 7 and 8, which are not in this tree yet).
* P10 (`python3 spec/tools/header_checks/s12.py [--include DIR] [--work DIR]`): spec/tools/p10_net.c (CMSG_*,
  IN6_IS_ADDR_*, the byte-order conversions) built on the system's headers and on libniji's (`-nostdinc`) must print the
  same bytes. The system's output is a diagnostic comparison, never read as text.
* enum (`nameser_enum_units`): the `enum __ns_*` enumerations of <arpa/nameser.h>, their members and typedefs.
Loaded by spec/tools/header_check.py, which lists this directory (plan/headers.md section 4, "Adding a slice").
"""
import os
import subprocess
import sys
import tempfile

ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".."))
P8_CONFIGS = ("gnu17", "default", "gnu", "c11", "cxx17")
INT_SUFFIX = {"int": "", "long": "L", "long long": "LL", "unsigned int": "U", "unsigned long": "UL", "unsigned long long": "ULL"}


def mine():
    """The headers of headers/slices/s12.tsv."""
    with open(os.path.join(ROOT, "headers", "slices", "s12.tsv")) as f:
        return [l.split("\t")[0] for l in f if l.strip() and not l.startswith("#")][1:]


def nameser_enum_units(ctx):
    """No spec clause beyond RFC 1035's field values (the type of the names by black-box probe, HEADERS-ENUM-FOLLOWUPS):
    per configuration that shows them, every enumeration `enum __ns_*` of <arpa/nameser.h> is declared with each of its
    enumerators.tsv members at its value, a member converts to its enumeration without a cast (C++: an int or a
    structure in its place fails), the typedefs `ns_type`, `ns_class`, `ns_sect` name those enumerations, and a DNS
    style switch over `ns_class` compiles under -Wall -Werror. Defects it catches: an enumeration left opaque, an
    enumeration of a reserved tag dropped with all its public members (`ns_r_servfail`), a wrong value."""
    tags = sorted(r["name"] for r in ctx.vis if r["header"] == "arpa/nameser.h" and r["kind"] == "enum" and r["name"].startswith("__ns_"))
    members = {}
    for r in ctx.rows("spec/abi/enumerators.tsv"):
        if r["tag"].startswith("enum ") and r["tag"][5:] in tags:
            members.setdefault(r["tag"][5:], []).append((r["name"], r["value"]))
    shown = {r["name"]: r["configs"] for r in ctx.vis if r["header"] == "arpa/nameser.h" and r["kind"] in ("enumerator", "enum", "typedef")}
    if len(tags) < 8 or set(members) != set(tags) or not {"ns_type", "ns_class", "ns_sect"} <= set(shown):
        return [(ctx.ids[0], "enum arpa/nameser.h", "#error the tables do not show the eight __ns_* enumerations and their typedefs\n")]
    lang = {c["id"]: c["language"] for c in ctx.cfgs}
    res = []
    for cid in ctx.ids:
        if cid not in expand(shown["ns_type"], ctx.ids):
            continue
        body = ["#include <arpa/nameser.h>", "ns_type niji_t = ns_t_a;", "ns_sect niji_s = ns_s_an;",
                "int niji_ns_f (ns_class c) { switch (c) { case ns_c_in: return 1; case ns_c_chaos: return 3; default: return 0; } }"]
        for tag in tags:
            for i, (n, v) in enumerate(members[tag]):
                if cid not in expand(shown.get(n, ""), ctx.ids):
                    continue
                body += [f"enum {tag} niji_{tag}_{i} = {n};", f"typedef char niji_v{tag}_{i}[({n}) == ({v}) ? 1 : -1];"]
            if lang[cid] == "c++":
                body.append(f"typedef char niji_k{tag}[__is_enum (enum {tag}) ? 1 : -1];")
        res.append((cid, "enum arpa/nameser.h", "\n".join(body) + "\n"))
    return res


def expand(configs, ids):
    if configs == "*":
        return set(ids)
    if configs.startswith("!"):
        return set(ids) - set(configs[1:].split(","))
    return set(configs.split(","))


def units(ctx):
    own = set(mine())
    others = {r["header"] for r in ctx.slice_rows if r["header"] not in own} | {"signal.h", "sys/param.h"}
    foreign = {r["name"] for r in ctx.vis if r["header"] in others}
    members = {}
    for r in ctx.rows("spec/abi/layouts.tsv"):
        if r["member"] and not r["bitfield"] and r["offset_or_size"].isdigit():
            members.setdefault(r["type"], []).append(r)
    out = {}
    for r in ctx.vis:
        h, n = r["header"], r["name"]
        if h not in own or r["kind"] not in ("macro", "typedef", "struct") or n in foreign:
            continue
        if n.startswith("__") or (n.startswith("_") and n.endswith("_H")):
            continue  # reserved names are emitted only where headers/reserved.tsv says so (owner rulings R-H2, R-H3)
        tests = []
        if r["kind"] == "macro" and n in ctx.macros and ctx.macros[n]["type"] in INT_SUFFIX and ctx.macros[n]["header"] == h:
            m = ctx.macros[n]
            tests.append(f"(({n}) == ({int(m['value'])}{INT_SUFFIX[m['type']]}))")
        elif r["kind"] == "typedef" and n in ctx.typedefs and ctx.typedefs[n]["size"]:
            t = ctx.typedefs[n]
            tests += [f"sizeof({n}) == {t['size']}", f"__alignof__({n}) == {t['align']}"]
        elif r["kind"] == "struct" and ("struct " + n) in ctx.layouts:
            t = ctx.layouts["struct " + n]
            tests += [f"sizeof(struct {n}) == {t['offset_or_size']}", f"__alignof__(struct {n}) == {t['align']}"]
            tests += [f"__builtin_offsetof(struct {n}, {m['member']}) == {m['offset_or_size']}" for m in members.get("struct " + n, [])]
        for cid in expand(r["configs"], ctx.ids):
            out.setdefault((h, cid), []).extend(tests)
    res = []
    for (h, cid), tests in out.items():
        if tests:
            body = f"#include <{h}>\n" + "".join(f"typedef char niji_p1_{i}[({t}) ? 1 : -1];\n" for i, t in enumerate(tests))
            res.append((cid, f"P1 {h}", body))
    res += member_names(ctx) + nameser_enum_units(ctx)
    # P8, libc-first only (ruling #6458: the kernel header first is not supported; slice 13 measures it): <netinet/in.h>
    # with linux/in6.h, once, with the shared tags used so a missing or doubled definition fails to compile. The other
    # kernel pairs are the `uapi` cells of headers/slices/s12.tsv, both orders.
    for cid in P8_CONFIGS:
        if cid in ctx.ids:
            res.append((cid, "P8 netinet/in.h then linux/in6.h", "#include <netinet/in.h>\n#include <linux/in6.h>\n"
                        "struct in_addr a; struct in6_addr b; struct sockaddr_in c; struct sockaddr_in6 d;\nint niji_p8_tu;\n"))
    return res


# netdevice(7): the member names of struct ifreq and struct ifconf, each a macro for a member of a union the layout
# table records (black-box form probe, HEADERS-BUSYBOX-GAPS: with the nested member's name made unusable after the
# header, the reference rejects a use of the macro). name: (tag, the nested member it names).
MEMBER_MACROS = {
    "ifr_name": ("ifreq", "ifr_ifrn.ifrn_name"),
    **{f"ifr_{m}": ("ifreq", f"ifr_ifru.ifru_{m}") for m in
       ("hwaddr", "addr", "dstaddr", "broadaddr", "netmask", "flags", "mtu", "map", "slave", "data", "newname")},
    **{f"ifr_{m}": ("ifreq", "ifr_ifru.ifru_ivalue") for m in ("metric", "ifindex", "bandwidth", "qlen")},
    "ifc_buf": ("ifconf", "ifc_ifcu.ifcu_buf"),
    "ifc_req": ("ifconf", "ifc_ifcu.ifcu_req"),
}


def member_names(ctx):
    """netdevice(7) member names of <net/if.h> (no spec clause beyond netdevice(7); HEADERS-BUSYBOX-GAPS): where
    visibility.tsv shows NAME, it is a macro for exactly the nested member MEMBER_MACROS records, so a use resolves in
    a structure that has only that path (the same offset alone would accept `ifr_ifindex` as `ifru_mtu`), and its
    offset in the real tag is the nested member's; where it does not, the header does not define it."""
    offs = {(r["type"], r["member"]): r["offset_or_size"] for r in ctx.rows("spec/abi/layouts.tsv") if r["member"]}
    shown, rows = {}, [r for r in ctx.vis if r["header"] == "net/if.h" and r["name"] in MEMBER_MACROS]
    if {r["name"] for r in rows} != set(MEMBER_MACROS):
        return [(ctx.ids[0], "member names net/if.h", "#error visibility.tsv and MEMBER_MACROS name different sets\n")]
    for r in rows:
        for cid in expand(r["configs"], ctx.ids):
            shown.setdefault(cid, []).append(r["name"])
    hdr = next(r for r in ctx.vis if r["header"] == "net/if.h" and r["name"] == "ifreq")
    res = []
    for cid in ctx.ids:
        names = sorted(shown.get(cid, []))
        if not names:
            if cid in expand(hdr["configs"], ctx.ids) or not any(cid in expand(r["configs"], ctx.ids) for r in ctx.vis if r["header"] == "net/if.h"):
                continue
            body = "#include <net/if.h>\n" + "".join(f"#ifdef {n}\n#error {n} is defined where struct {MEMBER_MACROS[n][0]} is not shown\n#endif\n" for n in MEMBER_MACROS)
            res.append((cid, "member names net/if.h (not shown)", body))
            continue
        body = ["#include <net/if.h>"]
        for i, n in enumerate(names):
            tag, path = MEMBER_MACROS[n]
            outer, leaf = path.split(".")
            body += [f"struct niji_mn{i} {{ struct {{ char {leaf}[{i + 2}]; }} {outer}; }};",
                     f"typedef char niji_mn{i}_form[sizeof(((struct niji_mn{i} *)0)->{n}) == {i + 2} ? 1 : -1];",
                     f"typedef char niji_mn{i}_off[__builtin_offsetof(struct {tag}, {n}) == {offs[('struct ' + tag, path)]} ? 1 : -1];"]
        res.append((cid, "member names net/if.h", "\n".join(body) + "\n"))
    return res


def p10(inc, work):
    src = os.path.join(ROOT, "spec", "tools", "p10_net.c")
    gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    outs = {}
    for tag, extra in (("reference", []), ("libniji", ["-nostdinc", "-isystem", inc, "-isystem", gccinc])):
        exe = os.path.join(work, "p10_" + tag)
        c = subprocess.run(["gcc", "-std=gnu17", "-O2", "-Wall", "-Wextra", "-Werror", *extra, src, "-o", exe], capture_output=True, text=True)
        if c.returncode:
            print(f"FAIL P10 {tag} build: {' | '.join(c.stderr.strip().splitlines()[:3])[:400]}")
            return 1
        outs[tag] = subprocess.run([exe], capture_output=True, text=True).stdout
    ok = outs["reference"] == outs["libniji"] and outs["reference"].count("\n") > 1000
    print(f"P10 net macros: {outs['reference'].count(chr(10))} lines, {'identical' if ok else 'DIFFERENT'}")
    if not ok:
        for i, (x, y) in enumerate(zip(outs["reference"].splitlines(), outs["libniji"].splitlines())):
            if x != y:
                print(f"FAIL P10 line {i}: reference `{x}` libniji `{y}`")
                break
    return 0 if ok else 1


if __name__ == "__main__":
    import argparse
    ap = argparse.ArgumentParser(description="P10 of slice 12")
    ap.add_argument("--include", default=os.path.join(ROOT, "libc-include"))
    ap.add_argument("--work", default="")
    a = ap.parse_args()
    w = a.work or tempfile.mkdtemp(prefix="p10.")
    os.makedirs(w, exist_ok=True)
    sys.exit(p10(os.path.abspath(a.include), w))
