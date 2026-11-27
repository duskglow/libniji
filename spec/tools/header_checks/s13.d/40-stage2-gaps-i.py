"""Slice 13 checks: stage2-gaps-i (header_checks/s13.d/, loaded after s13.py by spec/tools/header_check.py;
header_checks/README.md)."""
from header_check import expand, nested_spelling

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
#include <stddef.h>
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
# Item 1's form (gcc 14's fixincludes leaves a header alone that already has it), black-box probed against the
# reference: va_list is the compiler's __gnuc_va_list, requested through <stdarg.h>; the header defines
# _VA_LIST_DEFINED, and a program's own earlier definition of it suppresses the typedef (then `typedef int va_list`
# is the program's to make); <printf.h> gives the whole <stdarg.h>; size_t comes from the compiler's <stddef.h>
# (gcc's and clang's both define _SIZE_T for it).
VA_FORM = """#include <{h}>
#ifndef _VA_LIST_DEFINED
# error "_VA_LIST_DEFINED not defined"
#endif
__gnuc_va_list *niji_va_form = (va_list *) 0;
"""
VA_PREDEFINED = """#define _VA_LIST_DEFINED
#include <{h}>
typedef int va_list;
"""
PRINTF_STDARG = """#include <printf.h>
int niji_printf_va (int n, ...) { va_list a; va_start (a, n); int r = va_arg (a, int); va_end (a); return r; }
"""
SIZE_FORM = """#include <{h}>
#ifndef _SIZE_T
# error "size_t not from the compiler's <stddef.h>"
#endif
size_t niji_size_form;
"""
USES += [(h, "va_list", VA_FORM.format(h=h), "form ") for h in ("stdio.h", "wchar.h")]
USES += [(h, "va_list", VA_PREDEFINED.format(h=h), "predefined ") for h in ("stdio.h", "wchar.h")]
USES += [("printf.h", "va_list", PRINTF_STDARG, "stdarg ")]
# <ctype.h>'s _ISbit (libstdc++'s GNU locale model, ctype_members.cc): a function-like macro whose values the reference
# gives as below (black-box run, bits 0..15), usable in a constant expression.
ISBIT = """#include <ctype.h>
enum { niji_isbit = _ISbit (2) };
typedef char niji_isbit_values[_ISbit (0) == 256 && _ISbit (7) == 32768 && _ISbit (8) == 1 && _ISbit (15) == 128
                               && _ISbit (0) == _ISupper && _ISbit (11) == _ISalnum && niji_isbit == 1024 ? 1 : -1];
"""
USES += [("ctype.h", "_ISupper", ISBIT, "isbit ")]
USES += [(h, "size_t", SIZE_FORM.format(h=h), "form ") for h in ("stdlib.h", "unistd.h", "sys/types.h")]


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
    have the probed types (typed with _GNU_SOURCE and in C++, `void *` elsewhere); (3) a use in each consumer's shape,
    and the compiler-provided va_list and size_t forms of item 1 (gcc's fixincludes, libgomp, libstdc++'s configure).
    Defects: an opaque or reordered member, a callback typed as `void (*) (void)` (an incompatible-pointer error in
    gcc 14), a member-name macro of a union missing, struct termio only through <termio.h>."""
    lang = {c["id"]: c["language"] for c in ctx.cfgs}
    flags = {c["id"]: c["flags"] for c in ctx.cfgs}
    vis = {(r["header"], r["name"], r["kind"]): r["configs"] for r in ctx.vis}
    out = []
    for key, h, name, kind in TYPES:
        # A nested tag (headers/nested-tags.tsv, HEADERS-STAGE2-GAPS-J) through its outer structure's member.
        spell = nested_spelling(ctx, key) or key
        tests = [f"__builtin_offsetof ({spell}, {m}) == {o}" for m, o in offsets(ctx, key)]
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
    for h, name, body, *tag in USES:
        shown = set()
        for (vh, vn, _), cfgs in vis.items():
            if vh == h and vn == name:
                shown |= expand(cfgs, ctx.ids)
        out += [(cid, f"GAPS-I use {''.join(tag)}{name} {h}", body) for cid in ("gnu17", "default", "cxx98", "cxx17") if cid in shown]
    return out
