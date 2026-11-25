"""Slice 12 (plan/headers.md section 4: network core and name services): P1 of its sixteen headers with member offsets,
and P10, the network macros' output against the build host's own headers.

* P1  (`units(ctx)`): per (header, configuration) one unit that includes the header and asserts, for what it makes visible
  there, every macros.tsv integer value, every typedef and structure size and alignment, and the offset of every
  structure member that is not a bit-field. Names another slice's header owns are left to that header's P1 (signal.h and
  sys/param.h belong to slices 7 and 8, which are not in this tree yet).
* P10 (`python3 spec/tools/header_checks/s12.py [--include DIR] [--work DIR]`): spec/tools/p10_net.c (CMSG_*,
  IN6_IS_ADDR_*, the byte-order conversions) built on the system's headers and on libniji's (`-nostdinc`) must print the
  same bytes. The system's output is a diagnostic comparison, never read as text.
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
    # P8, libc-first only (ruling #6458: the kernel header first is not supported; slice 13 measures it): <netinet/in.h>
    # with linux/in6.h, once, with the shared tags used so a missing or doubled definition fails to compile. The other
    # kernel pairs are the `uapi` cells of headers/slices/s12.tsv, both orders.
    for cid in P8_CONFIGS:
        if cid in ctx.ids:
            res.append((cid, "P8 netinet/in.h then linux/in6.h", "#include <netinet/in.h>\n#include <linux/in6.h>\n"
                        "struct in_addr a; struct in6_addr b; struct sockaddr_in c; struct sockaddr_in6 d;\nint niji_p8_tu;\n"))
    # Spec clause: RFC 1035 4.1.1 (HEADERS-APT-GAPS: apt's SRV lookup): the DNS message header type `HEADER` exactly
    # where the reference shows it, with its size and alignment (P1 above; its bit positions are the rows test's).
    for h in ("arpa/nameser.h", "arpa/nameser_compat.h"):
        res += ctx.exact(h, ["HEADER"])
    # Spec clause: XSH <sys/socket.h> (accept, bind, connect, getpeername, getsockname, recvfrom, sendto; GNU accept4):
    # the address argument takes a `struct sockaddr *` (`const` for the const form) and a null pointer, in C++ too,
    # where the C transparent union does not exist (HEADERS-APT-GAPS: apt's methods, any C++ socket program).
    cfg = {c["id"]: c for c in ctx.cfgs}
    for cid in ctx.ids:
        if cfg[cid]["language"] != "c++":
            continue
        res.append((cid, "C++ socket calls with struct sockaddr *",
                    "#include <sys/socket.h>\n#include <netinet/in.h>\n"
                    "int niji_sock (int s, struct sockaddr *p, const struct sockaddr *c, socklen_t l, socklen_t *pl)\n"
                    "{\n  return connect (s, p, l) + connect (s, c, l) + bind (s, p, l) + bind (s, c, l)\n"
                    "    + accept (s, 0, 0) + accept (s, p, pl) + accept4 (s, p, pl, 0) + accept4 (s, 0, 0, 0)\n"
                    "    + getsockname (s, p, pl) + getpeername (s, p, pl)\n"
                    "    + (int) recvfrom (s, 0, 0, 0, p, pl) + (int) recvfrom (s, 0, 0, 0, 0, 0)\n"
                    "    + (int) sendto (s, 0, 0, 0, p, l) + (int) sendto (s, 0, 0, 0, c, l) + (int) sendto (s, 0, 0, 0, 0, 0);\n}\n"))
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
