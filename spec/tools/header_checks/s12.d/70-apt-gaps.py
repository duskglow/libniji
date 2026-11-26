"""Slice 12 checks: apt-gaps (header_checks/s12.d/, loaded after s12.py by spec/tools/header_check.py;
header_checks/README.md)."""


def units(ctx):
    res = []
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
