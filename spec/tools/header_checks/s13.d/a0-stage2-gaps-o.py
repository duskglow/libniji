"""Slice 13 checks: stage2-gaps-o (header_checks/s13.d/, loaded after s13.py by spec/tools/header_check.py;
header_checks/README.md). Yohane's R-H7 follow-up (HEADERS-STAGE2-GAPS-O; black-box probes of the reference, no header
text read): the public names the reference's headers define that libniji's lacked, which gcc's -fdump-go-spec dump
of libgo's sysinfo.c copies (mksysinfo.sh): <netinet/in.h> IPPORT_*, <net/route.h> RTCF_*, RTMSG_*, RT_CLASS_*,
<netinet/icmp6.h> and <netinet/ip_icmp.h> extended echo, <netinet/ip.h> IPTOS_DSCP_VA and struct timestamp, the
RFC 3678 structures, the <inttypes.h> C2X binary formats through the headers that include it, and the rest of
header_probe.d/stage2-gaps-o.py's list.

Stand-alone against the build host's own C library headers (the expectations are the reference's, so they hold there
too): python3 spec/tools/header_checks/s13.d/a0-stage2-gaps-o.py --reference
"""
import concurrent.futures
import importlib.util
import os
import re
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
from header_check import expand, kernel_flags  # noqa: E402

_HERE = os.path.dirname(os.path.abspath(__file__))
_FRAG = os.path.join(_HERE, "..", "..", "header_probe.d", "stage2-gaps-o.py")

# Names of the candidate list this story does not ship yet, each with the reason (handoff/HEADERS-STAGE2-GAPS-O.md).
# RT_*: the reference declares them in an enumeration inside struct r_debug's r_state member (C only; in C++ they are
# members of r_debug's scope), a construct the generator does not emit yet. The types: layouts not extracted yet.
NOT_YET = {"RT_ADD", "RT_CONSISTENT", "RT_DELETE"}
TYPES_SHIPPED = ("group_req", "group_filter", "ip_msfilter", "ip_opts", "timestamp")
# A name whose definition names a kernel UAPI macro: the dump keeps it only with that header included too.
WITH = {"RTMSG_OVERRUN": "linux/netlink.h"}
# The configurations of the Go-dump sweep (HEADERS-STAGE2-GAPS-N's six sets, by ftm-configs.tsv id where one matches).
GO_CONFIGS = {"gnu17": "-std=gnu17", "gnu": "-std=gnu17 -D_GNU_SOURCE", "c17": "-std=c17",
              "xopen700": "-std=c17 -D_XOPEN_SOURCE=700", "default": "-std=c17 -D_DEFAULT_SOURCE"}

IPPORT = (("ECHO", 7), ("DISCARD", 9), ("SYSTAT", 11), ("DAYTIME", 13), ("NETSTAT", 15), ("FTP", 21), ("TELNET", 23),
          ("SMTP", 25), ("TIMESERVER", 37), ("NAMESERVER", 42), ("WHOIS", 43), ("MTP", 57), ("TFTP", 69), ("RJE", 77),
          ("FINGER", 79), ("TTYLINK", 87), ("SUPDUP", 95), ("EXECSERVER", 512), ("LOGINSERVER", 513),
          ("CMDSERVER", 514), ("EFSSERVER", 520), ("BIFFUDP", 512), ("WHOSERVER", 513), ("ROUTESERVER", 520),
          ("RESERVED", 1024), ("USERRESERVED", 5000))
# The IPPORT_* names are enumerators of one enumeration, not macros (the probe: `#ifdef` fails, `enum { e = (N) }`
# compiles), with the reference's values.
IPPORT_UNIT = ("#include <netinet/in.h>\n"
               + "".join(f"#ifdef IPPORT_{n}\n#error \"IPPORT_{n} is a macro\"\n#endif\n" for n, _ in IPPORT)
               + "enum { niji_go_ipport = " + " && ".join(f"IPPORT_{n} == {v}" for n, v in IPPORT) + " ? 1 : -1 };\n"
               + "typedef char niji_go_ipport_t[niji_go_ipport];\n")
# RTMSG_OVERRUN is NLMSG_OVERRUN (an identical redefinition), 4 with <linux/netlink.h> in either order.
RTMSG_UNITS = ("#include <net/route.h>\n#define NLMSG_OVERRUN 99\n#if RTMSG_OVERRUN != 99\n#error \"RTMSG_OVERRUN is not NLMSG_OVERRUN\"\n#endif\n"
               "#undef NLMSG_OVERRUN\n#include <linux/netlink.h>\n"
               "typedef char niji_go_rtmsg[RTMSG_OVERRUN == 4 ? 1 : -1];\n",
               "#include <linux/netlink.h>\n#include <net/route.h>\n#if RTMSG_OVERRUN != 4\n#error\n#endif\n")
ROUTE_UNIT = """#include <net/route.h>
#if RTCF_VALVE != 0x00200000 || RTCF_LOG != 0x02000000 || RTMSG_NEWRULE != 0x31 || RTMSG_DELRULE != 0x32
#error "RTCF_ or RTMSG_ value"
#endif
#if RTMSG_CONTROL != 0x40 || RTMSG_AR_FAILED != 0x51 || RT_CLASS_UNSPEC != 0 || RT_CLASS_DEFAULT != 253
#error "RTMSG_ or RT_CLASS_ value"
#endif
#if RT_CLASS_MAIN != 254 || RT_CLASS_LOCAL != 255 || RT_CLASS_MAX != 255
#error "RT_CLASS_ value"
#endif
"""
# Sizes and the member offsets a program indexes (RFC 3678; the IP timestamp option of RFC 791).
STRUCT_UNIT = """#include <stddef.h>
#include <netinet/in.h>
#include <netinet/ip.h>
typedef char niji_go_s[sizeof (struct group_req) == 136 && offsetof (struct group_req, gr_group) == 8
  && sizeof (struct group_filter) == 272 && offsetof (struct group_filter, gf_fmode) == 136
  && offsetof (struct group_filter, gf_numsrc) == 140 && offsetof (struct group_filter, gf_slist) == 144
  && sizeof (struct ip_msfilter) == 20 && offsetof (struct ip_msfilter, imsf_slist) == 16
  && sizeof (struct ip_opts) == 44 && sizeof (((struct ip_opts *) 0)->ip_opts) == 40
  && sizeof (struct timestamp) == 40 && offsetof (struct timestamp, data) == 4 ? 1 : -1];
"""


def candidate_names():
    spec = importlib.util.spec_from_file_location("niji_gaps_o_candidates", _FRAG)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    names, tags = mod.candidates()
    return [n for n in names if n not in set(tags)], list(tags)


def units(ctx):
    """No spec clause beyond the reference's headers, RFC 3678 and RFC 791 (HEADERS-STAGE2-GAPS-O): in every
    configuration that shows them, (1) the IPPORT_* enumerators of <netinet/in.h>, values and not macros; (2) the
    <net/route.h> RTCF_*, RTMSG_*, RT_CLASS_* values in #if, and RTMSG_OVERRUN as NLMSG_OVERRUN; (3) the sizes and
    offsets of struct group_req, group_filter, ip_msfilter, ip_opts and timestamp. Defects: a name missing, a value
    or kind changed (an enumerator as a macro), a member moved."""
    shown = {}
    for r in ctx.vis:
        shown.setdefault((r["header"], r["name"]), set()).update(expand(r["configs"], ctx.ids))
    out = [(cid, "GAPS-O IPPORT", IPPORT_UNIT) for cid in sorted(shown.get(("netinet/in.h", "IPPORT_ECHO"), ()))]
    for cid in sorted(shown.get(("net/route.h", "RTCF_LOG"), ())):
        out.append((cid, "GAPS-O route values", ROUTE_UNIT))
        out += [(cid, f"GAPS-O RTMSG_OVERRUN {i}", u) for i, u in enumerate(RTMSG_UNITS)]
    for cid in sorted(shown.get(("netinet/in.h", "group_filter"), set()) & shown.get(("netinet/ip.h", "timestamp"), set())):
        out.append((cid, "GAPS-O structures", STRUCT_UNIT))
    return out


def _cmd(ctx, lang, flags, work):
    cc = "g++" if lang == "c++" else "gcc"
    cmd = [cc, *flags.split()]
    if ctx.inc:
        own = subprocess.run([cc, "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
        cmd += ["-nostdinc", "-isystem", ctx.inc, "-isystem", own, *kernel_flags(work)]
    return cmd


def _cc(ctx, lang, flags, body, work, extra=()):
    src = os.path.join(work, "go.cc" if lang == "c++" else "go.c")
    with open(src, "w") as f:
        f.write(body)
    r = subprocess.run(_cmd(ctx, lang, flags, work) + ["-fsyntax-only", *extra, src], capture_output=True, text=True,
                       timeout=300)
    return r.returncode, r.stderr


CONST = re.compile(r"^const _(\w+) = ", re.M)


def problems(ctx):
    """No spec clause beyond the reference's headers and gcc's -fdump-go-spec (HEADERS-STAGE2-GAPS-O): every shipped
    name of header_probe.d/stage2-gaps-o.py's list (all but NOT_YET; the types of TYPES_SHIPPED) is a `const` of the
    Go dump of each header that shows it, alone, in each sweep configuration that shows it (the `nameonly` class of
    the GAPS-N sweep for this list: expected 0). A macro, enumerator or size gcc cannot dump fails here (the libgo
    syscall package would lose it). Defects: a name missing or defined in a form the dump drops."""
    names, _ = candidate_names()
    want = {}
    for r in ctx.vis:
        n, k = r["name"], r["kind"]
        if r["header"] == "-" or n in NOT_YET:
            continue
        if (k in ("macro", "enumerator") and n in names) or (k == "struct" and n in TYPES_SHIPPED):
            key = n if k != "struct" else f"sizeof_{n}"
            for cid in set(expand(r["configs"], ctx.ids)) & set(GO_CONFIGS):
                want.setdefault((r["header"], cid), set()).add(key)
    out = []
    with tempfile.TemporaryDirectory(prefix="niji-gaps-o-") as work:
        _cmd(ctx, "c", "", work)  # the kernel UAPI directory once, before the threads use it

        def run(job):
            (h, cid), keys = job
            extra = sorted({WITH[k] for k in keys if k in WITH})
            tag = f"{h.replace('/', '_')}-{cid}"
            src, go = os.path.join(work, tag + ".c"), os.path.join(work, tag + ".go")
            with open(src, "w") as f:
                f.write("".join(f"#include <{x}>\n" for x in extra + [h]))
            r = subprocess.run(_cmd(ctx, "c", GO_CONFIGS[cid], work) + [f"-fdump-go-spec={go}", "-c", src, "-o", os.devnull],
                               capture_output=True, text=True, timeout=300)
            if r.returncode != 0 or not os.path.exists(go):
                return [f"GAPS-O go dump [{h} {cid}]: rc {r.returncode}: {r.stderr.strip()[:200]}"]
            got = set(CONST.findall(open(go).read()))
            return [f"GAPS-O go dump [{h} {cid}]: no `const _{k}`" for k in sorted(keys - got)]
        with concurrent.futures.ThreadPoolExecutor(max_workers=min(16, os.cpu_count() or 2)) as ex:
            for bad in ex.map(run, sorted(want.items())):
                out += bad
    seen = {k for ks in want.values() for k in ks}
    for n in [x for x in names if x not in NOT_YET] + [f"sizeof_{t}" for t in TYPES_SHIPPED]:
        if n not in seen:
            out.append(f"GAPS-O go dump: {n} has no visibility row in a sweep configuration")
    if not want:
        out.append("GAPS-O go dump: no shipped name has a visibility row (table or list lost)")
    return out


if __name__ == "__main__":
    import types
    import csv
    root = os.path.join(_HERE, "..", "..", "..", "..")

    def tsv(rel):
        with open(os.path.join(root, rel), newline="") as f:
            return list(csv.DictReader((x for x in f if not x.startswith("#")), delimiter="\t"))
    cfgs = tsv("spec/abi/ftm-configs.tsv")
    ref = "--reference" in sys.argv
    ctx = types.SimpleNamespace(ids=[c["id"] for c in cfgs], cfgs=cfgs, vis=tsv("spec/abi/visibility.tsv"),
                                inc=None if ref else os.path.abspath(os.path.join(root, "libc-include")))
    by_id = {c["id"]: c for c in cfgs}
    rejected = {r["id"] for r in tsv("headers/levels.tsv") if r["reject"] == "1"}
    fails = problems(ctx)
    with tempfile.TemporaryDirectory(prefix="niji-gaps-o-u-") as w:
        for cid, what, body in units(ctx):
            if cid in rejected:
                continue
            c = by_id[cid]
            rc, err = _cc(ctx, c["language"], c["flags"] + (" -Wno-cpp" if cid in ("bsd", "svid") else ""), body, w)
            if rc:
                fails.append(f"{what} {cid}: rc {rc}: {' | '.join(err.strip().splitlines()[:3])[:300]}")
    for f in fails:
        print(f"FAIL {f}")
    print(f"gaps-o: {len(fails)} failed")
    sys.exit(1 if fails else 0)
