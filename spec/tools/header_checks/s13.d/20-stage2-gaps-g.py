"""Slice 13 checks: stage2-gaps-g (header_checks/s13.d/, loaded after s13.py by spec/tools/header_check.py;
header_checks/README.md)."""


def units(ctx):
    """No spec clause beyond ethernet(7) and the kernel's UAPI (black-box probe of the reference's include closure,
    HEADERS-STAGE2-GAPS-G): where visibility.tsv shows them, <netinet/if_ether.h> (through <net/ethernet.h>) has the
    kernel's complete struct ethhdr (iptables) and <netinet/if_fddi.h> the kernel's struct fddihdr and FDDI_K_* sizes
    (net-tools). Defects: the structure left incomplete or defined a second time beside the kernel's (a redefinition
    in every program that includes both), a missing size macro."""
    vis = {}
    for r in ctx.vis:
        if r["header"] in ("netinet/if_ether.h", "netinet/if_fddi.h"):
            vis[(r["header"], r["name"])] = ctx.expand(r["configs"])
    out = []
    for cid in ctx.ids:
        if cid in vis.get(("netinet/if_ether.h", "ethhdr"), ()):
            out.append((cid, "ethhdr through netinet/if_ether.h", "#include <netinet/if_ether.h>\n#include <linux/if_ether.h>\n"
                        "typedef char niji_eh[sizeof (struct ethhdr) == 14 ? 1 : -1];\n"
                        "int niji_ep (const struct ethhdr *e) { return e->h_proto + e->h_dest[0] + e->h_source[0]; }\n"))
        if cid in vis.get(("netinet/if_fddi.h", "FDDI_K_ALEN"), ()):
            out.append((cid, "FDDI sizes", "#include <netinet/if_fddi.h>\n"
                        "typedef char niji_fd[FDDI_K_ALEN == 6 && sizeof (struct fddihdr) > 0 ? 1 : -1];\n"))
    return out
