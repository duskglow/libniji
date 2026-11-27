"""Slice 13 checks: stage2-gaps-i (header_checks/s13.d/, loaded after s13.py by spec/tools/header_check.py;
header_checks/README.md)."""
from header_check import expand

# (layouts.tsv type, header, visibility.tsv name, kind): the structures HEADERS-STAGE2-GAPS-I gives members (opaque
# before, or new), checked in the header a consumer includes. struct termio also through <sys/ioctl.h> (llvm-19's
# sanitizer takes its size there).
TYPES = [
    ("glob64_t", "glob.h", "glob64_t", "typedef"),
    ("fpos64_t", "stdio.h", "fpos64_t", "typedef"),
    ("struct aiocb64", "aio.h", "aiocb64", "struct"),
    ("struct igmp", "netinet/igmp.h", "igmp", "struct"),
    ("struct termio", "termio.h", "termio", "struct"),
    ("struct termio", "sys/ioctl.h", "termio", "struct"),
    ("struct osockaddr", "sys/socket.h", "osockaddr", "struct"),
    ("CTL_MSG", "protocols/talkd.h", "CTL_MSG", "typedef"),
    ("CTL_RESPONSE", "protocols/talkd.h", "CTL_RESPONSE", "typedef"),
    ("struct whod", "protocols/rwhod.h", "whod", "struct"),
    ("struct whoent", "protocols/rwhod.h", "whoent", "struct"),
    ("struct outmp", "protocols/rwhod.h", "outmp", "struct"),
    ("struct tsp", "protocols/timed.h", "tsp", "struct"),
    ("struct rip", "protocols/routed.h", "rip", "struct"),
    ("struct netinfo", "protocols/routed.h", "netinfo", "struct"),
]

# The GLOB_ALTDIRFUNC callbacks (black-box probe of the reference): typed with _GNU_SOURCE, `void *` elsewhere.
GLOB = {
    "glob_t": ("struct dirent", "struct stat"),
    "glob64_t": ("struct dirent64", "struct stat64"),
}

USES = [
    ("protocols/talkd.h", "CTL_MSG", """#include <protocols/talkd.h>
#include <string.h>
#include <arpa/inet.h>
int niji_talk_use (CTL_MSG *m, const CTL_RESPONSE *r, const char *who)
{
  m->vers = TALK_VERSION;
  m->type = LOOK_UP;
  m->id_num = htonl (0);
  m->addr.sa_family = htons (AF_INET);
  m->ctl_addr = m->addr;
  m->pid = 1;
  strncpy (m->l_name, who, NAME_SIZE);
  strncpy (m->r_tty, who, TTY_SIZE);
  return r->answer == SUCCESS || r->answer == NOT_HERE || r->type == ANNOUNCE || r->type == DELETE
         || r->type == LEAVE_INVITE || r->answer == BADVERSION;
}
typedef char niji_talk_sizes[sizeof (((CTL_MSG *) 0)->l_name) == NAME_SIZE && sizeof (((CTL_MSG *) 0)->r_tty) == TTY_SIZE ? 1 : -1];
"""),
    ("protocols/timed.h", "tsp_time", """#include <protocols/timed.h>
long niji_tsp_use (struct tsp *t)
{
  t->tsp_type = TSP_LOOP;
  t->tsp_vers = TSPVERSION;
  t->tsp_hopcnt = 4;
  return (long) t->tsp_time.tv_sec + (ANYADDR == 0) + TSPTYPENUMBER;
}
"""),
    ("protocols/routed.h", "rip_nets", """#include <protocols/routed.h>
int niji_rip_use (struct rip *r)
{
  r->rip_cmd = RIPCMD_REQUEST;
  r->rip_vers = RIPVERSION;
  r->rip_nets[0].rip_metric = HOPCNT_INFINITY;
  return r->rip_tracefile[0] + MAXPACKETSIZE;
}
"""),
    ("sys/ioctl.h", "termio", """#include <sys/ioctl.h>
unsigned niji_termio_size = sizeof (struct termio);
int niji_termio_use (struct termio *t) { return t->c_cc[0] + t->c_line + t->c_lflag; }
"""),
    ("netinet/igmp.h", "igmp", """#include <netinet/igmp.h>
int niji_igmp_use (const struct igmp *g) { return g->igmp_type + g->igmp_code + g->igmp_cksum + (int) g->igmp_group.s_addr; }
"""),
    ("stdlib.h", "comparison_fn_t", """#include <stdlib.h>
static int niji_cmp (const void *a, const void *b) { return *(const int *) a - *(const int *) b; }
void niji_sort (int *v, size_t n) { comparison_fn_t f = niji_cmp; qsort (v, n, sizeof *v, f); }
"""),
]


def offsets(ctx, key):
    """The members of `key` whose offset a program can name: plain members, and members below a named member that is
    not an array (`tsp_u.tspu_time`); no bit-fields."""
    rows = [r for r in ctx.rows("spec/abi/layouts.tsv") if r["type"] == key and r["member"]]
    kind = {r["member"]: r["member_type"] for r in rows}
    out = []
    for r in rows:
        m = r["member"]
        parts = m.split(".")
        if r["bitfield"] or any(kind.get(".".join(parts[:i]), "").endswith("]") for i in range(1, len(parts))):
            continue
        out.append((m, r["offset_or_size"]))
    return out


def units(ctx):
    """No spec clause beyond the reference's layouts (HEADERS-STAGE2-GAPS-I, Yohane stage 2 on 26.3: fakechroot
    glob64.c, nftables' igmp, llvm-19's sanitizer, inetutils' talk and talkd, systemd's sort-util.h): (1) in each
    configuration where visibility.tsv shows the type in the header, `__builtin_offsetof` of every member layouts.tsv
    records (the shared P1 check has size and alignment only); (2) glob_t's and glob64_t's GLOB_ALTDIRFUNC callbacks
    have the probed types (typed with _GNU_SOURCE and in C++, `void *` elsewhere); (3) a use in each consumer's shape.
    Defects: an opaque or reordered member, a callback typed as `void (*) (void)` (an incompatible-pointer error in
    gcc 14), a member-name macro of a union missing, struct termio only through <termio.h>."""
    lang = {c["id"]: c["language"] for c in ctx.cfgs}
    flags = {c["id"]: c["flags"] for c in ctx.cfgs}
    vis = {(r["header"], r["name"], r["kind"]): r["configs"] for r in ctx.vis}
    out = []
    for key, h, name, kind in TYPES:
        tests = [f"__builtin_offsetof ({key}, {m}) == {o}" for m, o in offsets(ctx, key)]
        if not tests:
            continue
        body = f"#include <{h}>\n" + "".join(f"typedef char niji_gi_off_{i}[({t}) ? 1 : -1];\n" for i, t in enumerate(tests))
        for cid in sorted(expand(vis.get((h, name, kind), ""), ctx.ids)):
            out.append((cid, f"GAPS-I offsets {key} {h}", body))
    for ty, (dirent, stat) in GLOB.items():
        for cid in sorted(expand(vis.get(("glob.h", ty, "typedef"), ""), ctx.ids)):
            cxx = lang[cid] == "c++"
            typed = cxx or "-D_GNU_SOURCE" in flags[cid].split()
            d, s = (dirent, stat) if typed else ("void", "void")
            same = (lambda a, b: f"__is_same (__typeof__ ({a}), {b})") if cxx else (lambda a, b: f"__builtin_types_compatible_p (__typeof__ ({a}), {b})")
            m = f"(({ty} *) 0)->"
            tests = [same(m + "gl_closedir", "void (*) (void *)"), same(m + "gl_readdir", f"{d} *(*) (void *)"),
                     same(m + "gl_opendir", "void *(*) (const char *)"), same(m + "gl_lstat", f"int (*) (const char *, {s} *)"),
                     same(m + "gl_stat", f"int (*) (const char *, {s} *)")]
            body = "#include <glob.h>\n" + "".join(f"typedef char niji_gi_glob_{i}[({t}) ? 1 : -1];\n" for i, t in enumerate(tests))
            out.append((cid, f"GAPS-I callbacks {ty}", body))
    for h, name, body in USES:
        shown = set()
        for (vh, vn, _), cfgs in vis.items():
            if vh == h and vn == name:
                shown |= expand(cfgs, ctx.ids)
        out += [(cid, f"GAPS-I use {name} {h}", body) for cid in ("gnu17", "default", "cxx98", "cxx17") if cid in shown]
    return out
