"""The kernel's UAPI protocol (linux/libc-compat.h, kernel 6.8; plan/headers.md section 3, P8): which definitions are in
force after a kernel UAPI header and a libniji header in each include order. P8 proves that each pair compiles in both
orders; these units use what the pair defines. No spec clause: the protocol is the kernel's (include/uapi/linux/
libc-compat.h, include/uapi/linux/in6.h, include/uapi/linux/ipv6.h); the expectations were settled by compile-only
probes of the reference system (HEADERS-UAPI-COMPAT).

* Kernel header first (linux/in6.h, linux/ipv6.h): the kernel's struct in6_addr (member in6_u), its s6_addr,
  s6_addr16 and s6_addr32 (in every configuration), its struct ipv6_mreq (member ipv6mr_ifindex), and linux/ipv6.h's
  struct in6_pktinfo and struct ip6_mtuinfo; libniji's IN6_* tests and inet_pton still work on them.
* libniji header first: libniji's definitions (ipv6mr_interface), which the kernel's headers then leave alone.
* linux/if.h first: it includes linux/libc-compat.h but defines no IPv6 structure, so libniji's are in force.
Loaded by spec/tools/header_check.py, which lists this directory.
"""
import os

CONFIGS = ("c17", "gnu17", "gnu", "cxx17")
ASSERT = "#ifdef __cplusplus\n# define NIJI_SA static_assert\n#else\n# define NIJI_SA _Static_assert\n#endif\n"
UNITS = [
    ("uapi linux/in6.h then netinet/in.h", "#include <linux/in6.h>\n#include <netinet/in.h>\n#include <arpa/inet.h>\n" + ASSERT +
     "NIJI_SA (sizeof (struct in6_addr) == 16, \"in6_addr\");\nNIJI_SA (sizeof (struct sockaddr_in6) == 28, \"sockaddr_in6\");\n"
     "int niji_uapi (struct in6_addr *a, struct sockaddr_in6 *s, struct ipv6_mreq *m)\n{\n"
     "  a->in6_u.u6_addr8[0] = a->s6_addr[1];\n  a->s6_addr16[1] = 0;\n  a->s6_addr32[1] = 0;\n"
     "  s->sin6_scope_id = 0;\n  m->ipv6mr_ifindex = 0;\n"
     "  return IN6_IS_ADDR_LOOPBACK (a) + IN6_ARE_ADDR_EQUAL (a, &s->sin6_addr) + inet_pton (AF_INET6, \"::1\", a);\n}\n"),
    ("uapi linux/ipv6.h then netinet/ip6.h", "#include <linux/ipv6.h>\n#include <netinet/ip6.h>\n"
     "int niji_uapi (struct in6_pktinfo *p, struct ip6_mtuinfo *u, struct ip6_hdr *h)\n{\n"
     "  p->ipi6_ifindex = 0;\n  u->ip6m_mtu = 0;\n  p->ipi6_addr.s6_addr32[0] = h->ip6_src.s6_addr32[0];\n  return IN6_IS_ADDR_MULTICAST (&h->ip6_dst);\n}\n"),
    ("uapi netinet/in.h then linux/ipv6.h", "#include <netinet/in.h>\n#include <linux/ipv6.h>\n"
     "int niji_uapi (struct in6_addr *a, struct ipv6_mreq *m)\n{\n  m->ipv6mr_interface = 0;\n  a->s6_addr[0] = 0;\n"
     "  return IN6_IS_ADDR_UNSPECIFIED (&m->ipv6mr_multiaddr);\n}\n"),
    ("uapi linux/if.h then netinet/in.h", "#include <linux/if.h>\n#include <netinet/in.h>\n"
     "int niji_uapi (struct in6_addr *a, struct ipv6_mreq *m)\n{\n  m->ipv6mr_interface = 0;\n  a->s6_addr[0] = 0;\n"
     "  return IN6_IS_ADDR_LOOPBACK (a);\n}\n"),
]


def kernel_headers():
    """Whether the kernel UAPI headers these units include are reachable ($NIJI_UAPI or the build host's)."""
    base = os.environ.get("NIJI_UAPI") or "/usr/include"
    return all(os.path.isfile(os.path.join(base, h)) for h in ("linux/in6.h", "linux/ipv6.h", "linux/if.h"))


def units(ctx):
    if not kernel_headers():
        print("skip uapi units: no kernel UAPI headers (linux/in6.h, linux/ipv6.h, linux/if.h) on this host")
        return []
    return [(cid, what, body + "int niji_check_tu;\n") for cid in CONFIGS if cid in ctx.ids for what, body in UNITS]
