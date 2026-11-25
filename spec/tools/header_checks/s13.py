"""Slice 13 (plan/headers.md section 4: networking): the checks only its headers need.

* member offsets of the amateur-radio and legacy protocol headers (HEADERS-STAGE2-GAPS-C; net-tools names `struct
  sockaddr_ax25`): for each structure or typedef netax25/ax25.h, netrom/netrom.h, netrose/rose.h, netipx/ipx.h,
  netash/ash.h and neteconet/ec.h make visible, `__builtin_offsetof` of every plain member of layouts.tsv (the shared
  P1 checks size and alignment only). Spec clause: the kernel's socket address structures of these families (the
  kernel UAPI headers linux/ax25.h, linux/netrom.h, linux/rose.h), read by the kernel at these offsets;
* a use in the shape net-tools' AX.25 and ROSE address code has, C and C++: a callsign copied into `sax25_call`, the
  digipeater list of `struct full_sockaddr_ax25` and a ROSE address, sized as the kernel reads them.
Loaded by spec/tools/header_check.py, which lists this directory.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import header_check as hc  # noqa: E402

RADIO = ["netax25/ax25.h", "netrom/netrom.h", "netrose/rose.h", "netipx/ipx.h", "netash/ash.h", "neteconet/ec.h"]

USE = """#include <netax25/ax25.h>
#include <netrose/rose.h>
#include <string.h>
#include <sys/socket.h>
int niji_ax25_use (struct full_sockaddr_ax25 *fsa, struct sockaddr_rose *sr, const char *call)
{
  struct sockaddr_ax25 *sa = &fsa->fsa_ax25;
  ax25_address *digi = &fsa->fsa_digipeater[AX25_MAX_DIGIS - 1];
  sa->sax25_family = AF_AX25;
  memcpy (sa->sax25_call.ax25_call, call, sizeof sa->sax25_call.ax25_call);
  memcpy (digi->ax25_call, call, sizeof digi->ax25_call);
  sa->sax25_ndigis = 1;
  sr->srose_family = AF_ROSE;
  memcpy (sr->srose_addr.rose_addr, call, sizeof sr->srose_addr.rose_addr);
  sr->srose_call = sa->sax25_call;
  return SOL_AX25 + ROSE_MAX_DIGIS;
}
typedef char niji_ax25_call[sizeof (ax25_address) == 7 && sizeof (rose_address) == 5 ? 1 : -1];
typedef char niji_ax25_sa[sizeof (struct sockaddr_ax25) == 16 && sizeof (struct full_sockaddr_ax25) == 16 + 7 * AX25_MAX_DIGIS ? 1 : -1];
"""

# HEADERS-STAGE2-GAPS-C-2 (completeness pass over thin headers): uses of the names it added, each only in the
# configurations visibility.tsv shows the name in. Spec clauses: C23 7.8.1 (PRIb*, SCNb*: the binary conversions of
# <inttypes.h>); udp(7) (the UDP socket options); utmp(5) (the older member names of a login record).
THIN = [
    ("inttypes.h", "PRIb64", """#include <inttypes.h>
typedef char niji_thin_pri[sizeof (PRIb64) == 3 && PRIb64[0] == 'l' && PRIb64[1] == 'b' && sizeof (PRIB32) == 2 && PRIB32[0] == 'B'
                           && sizeof (SCNb8) == 4 && SCNb8[0] == 'h' && SCNb8[1] == 'h' && SCNb8[2] == 'b'
                           && sizeof (PRIbPTR) == 3 && PRIbPTR[0] == 'l' && PRIbPTR[1] == 'b' ? 1 : -1];
"""),
    ("netinet/udp.h", "UDP_SEGMENT", """#include <netinet/udp.h>
typedef char niji_thin_udp[UDP_CORK == 1 && UDP_ENCAP == 100 && UDP_SEGMENT == 103 && UDP_GRO == 104 && UDP_NO_CHECK6_TX == 101
                           && UDP_ENCAP_ESPINUDP == 2 && SOL_UDP == 17 ? 1 : -1];
"""),
    ("utmp.h", "ut_xtime", """#include <utmp.h>
#include <string.h>
typedef char niji_thin_ut[__builtin_offsetof (struct utmp, ut_name) == __builtin_offsetof (struct utmp, ut_user)
                          && __builtin_offsetof (struct utmp, ut_time) == __builtin_offsetof (struct utmp, ut_tv)
                          && __builtin_offsetof (struct utmp, ut_xtime) == __builtin_offsetof (struct utmp, ut_tv)
                          && __builtin_offsetof (struct utmp, ut_addr) == __builtin_offsetof (struct utmp, ut_addr_v6)
                          && sizeof ((struct utmp *) 0)->ut_name == 32 && sizeof ((struct utmp *) 0)->ut_time == 4 ? 1 : -1];
int niji_thin_utmp (struct utmp *u)
{
  u->ut_time = 5;
  u->ut_xtime += 1;
  u->ut_addr = 7;
  return (int) strlen (u->ut_name) + (int) u->ut_tv.tv_sec + (int) u->ut_addr_v6[0] + (&u->ut_name[0] == &u->ut_user[0]) + (UTMP_FILENAME[0] == '/');
}
"""),
    ("arpa/nameser.h", "ns_rr_ttl", """#include <arpa/nameser.h>
typedef char niji_thin_ns[sizeof (ns_msg_id (*(ns_msg *) 0)) == sizeof (int) && sizeof (ns_msg_size (*(ns_msg *) 0)) == sizeof (long)
                          && sizeof (ns_msg_count (*(ns_msg *) 0, ns_s_an)) == sizeof (int)
                          && sizeof (ns_rr_ttl (*(ns_rr *) 0)) == sizeof (unsigned int) && sizeof (ns_rr_rdlen (*(ns_rr *) 0)) == sizeof (int)
                          && sizeof (ns_rr_name (*(ns_rr *) 0)) == sizeof (char *) && sizeof (ns_rr_type (*(ns_rr *) 0)) == sizeof (ns_type)
                          && sizeof (NS_TSIG_ALG_HMAC_MD5) == 25 ? 1 : -1];
int niji_thin_nsuse (const unsigned char *cp, unsigned char *op, ns_msg *m, ns_rr *rr)
{
  unsigned short s;
  unsigned long l;
  GETSHORT (s, cp);
  GETLONG (l, cp);
  NS_GET16 (s, cp);
  NS_GET32 (l, cp);
  PUTSHORT (s, op);
  PUTLONG (l, op);
  NS_PUT16 (s, op);
  NS_PUT32 (l, op);
  return ns_msg_id (*m) + (int) (ns_msg_end (*m) - ns_msg_base (*m)) + (int) ns_rr_type (*rr) + (int) ns_rr_class (*rr)
         + (int) ns_rr_rdlen (*rr) + (ns_rr_name (*rr)[0] == 0) + (ns_rr_rdata (*rr) != 0) + (int) ns_rr_ttl (*rr);
}
"""),
]


def members(ctx):
    out = {}
    for r in ctx.rows("spec/abi/layouts.tsv"):
        m = r["member"]
        if m and "." not in m and "<" not in m and not r["bitfield"] and "<anon>" not in r["member_type"]:
            out.setdefault(r["type"], []).append((m, r["offset_or_size"]))
    return out


def units(ctx):
    mem = members(ctx)
    # Only the types these headers own (typedefs.tsv): what they show through <sys/types.h> (the pthread internals)
    # is checked by the slices of those headers.
    home = {(r["kind"], r["name"]): r["header"] for r in ctx.rows("spec/abi/typedefs.tsv")}
    per = {}
    for r in ctx.vis:
        h = r["header"]
        if h not in RADIO or r["kind"] not in ("typedef", "struct"):
            continue
        n = r["name"]
        if home.get((r["kind"], n)) not in RADIO:
            continue
        key = n if r["kind"] == "typedef" else "struct " + n
        if not mem.get(key) or (r["kind"] == "struct" and key not in ctx.layouts):
            continue
        tests = [f"__builtin_offsetof({key}, {m}) == {o}" for m, o in mem[key]]
        for cid in hc.expand(r["configs"], ctx.ids):
            per.setdefault((h, cid), []).extend(tests)
    out = []
    for (h, cid), tests in sorted(per.items()):
        body = f"#include <{h}>\n" + "".join(f"typedef char niji_s13_off_{i}[({t}) ? 1 : -1];\n" for i, t in enumerate(tests))
        out.append((cid, f"S13 offsets {h}", body))
    for h, name, body in THIN:
        for r in ctx.vis:
            if r["header"] == h and r["name"] == name and r["kind"] == "macro":
                out += [(cid, f"S13 thin {h}", body) for cid in ("gnu17", "gnu2x", "cxx17") if cid in hc.expand(r["configs"], ctx.ids)]
    out += [(cid, "S13 ax25 use", USE) for cid in ("gnu17", "c99", "cxx98", "cxx17") if cid in ctx.ids]
    return out
