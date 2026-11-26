"""Slice 12 checks: stage2-gaps-g (header_checks/s12.d/, loaded after s12.py by spec/tools/header_check.py;
header_checks/README.md)."""
import glob
import os
import re


def renames(ctx):
    """The `#define x __x` renames of <resolv.h>'s template and fragments: (x, __x)."""
    text = "".join(open(p).read() for p in [os.path.join(ctx.root, "headers/resolv.h.in")]
                   + sorted(glob.glob(os.path.join(ctx.root, "headers/resolv.h.d/*.frag"))))
    return sorted(set(re.findall(r"^#\s*define\s+(\w+)\s+__(\1)\s*$", text, re.M)))


def units(ctx):
    """No spec clause (black-box probe of the reference, HEADERS-STAGE2-GAPS-G: openssh's openbsd-compat/base64.h
    defines `__b64_ntop(a,b,c,d)` as a call of b64_ntop before it includes <resolv.h>): for each rename `x` -> `__x`
    of <resolv.h>, where visibility.tsv shows `__x` there, a function-like macro `__x` defined before the header
    leaves the header's declaration of `__x` intact (the name, not followed by `(`, is a declared function after the
    include) and a call through the macro compiles. Defects: a declarator `__x (` that the program's macro rewrites
    into a declaration of `x` (openssh: implicit declaration of __b64_ntop)."""
    out = []
    pairs = renames(ctx)
    vis = {r["name"]: r["configs"] for r in ctx.vis if r["header"] == "resolv.h" and r["kind"] == "function"}
    for cid in ctx.ids:
        here = [(x, "__" + x) for x, _ in pairs if "__" + x in vis and cid in ctx.expand(vis["__" + x])]
        if not here:
            continue
        body = [f"#define {t}(...) {x} (__VA_ARGS__)" for x, t in here] + ["#include <resolv.h>"]
        body += [f"void *niji_rn{i} = (void *) ({t});" for i, (_, t) in enumerate(here)]
        out.append((cid, "resolv.h renames after a program's macros", "\n".join(body) + "\n"))
    out += ntypes(ctx)
    return out


def ntypes(ctx):
    """No spec clause (black-box probe of the reference, RELEASE-SNAPSHOT-1126B: inetutils ping's icmp.h uses n_time):
    <netinet/in_systm.h> declares n_short as uint16_t and n_long and n_time as uint32_t (the same types, not only the
    same sizes), in each configuration visibility.tsv shows them, C and C++. Defects: a missing name, or a type of
    another signedness or width (an icmp timestamp compared with a signed value)."""
    vis = {r["name"]: r["configs"] for r in ctx.vis if r["header"] == "netinet/in_systm.h" and r["kind"] == "typedef"}
    want = {"n_short": "uint16_t", "n_long": "uint32_t", "n_time": "uint32_t"}
    out = []
    for cid in ctx.ids:
        here = [n for n in want if n in vis and cid in ctx.expand(vis[n])]
        if not here:
            continue
        body = ["#include <netinet/in_systm.h>", "#include <stdint.h>", "#ifdef __cplusplus",
                "template <class A, class B> struct niji_same { enum { v = 0 }; };",
                "template <class A> struct niji_same<A, A> { enum { v = 1 }; };",
                "# define NIJI_SAME(a, b) niji_same<a, b>::v", "#else",
                "# define NIJI_SAME(a, b) __builtin_types_compatible_p (a, b)", "#endif"]
        body += [f"typedef char niji_nt_{n}[NIJI_SAME ({n}, {want[n]}) ? 1 : -1];" for n in here]
        out.append((cid, "netinet/in_systm.h n_short n_long n_time types", "\n".join(body) + "\n"))
    return out
