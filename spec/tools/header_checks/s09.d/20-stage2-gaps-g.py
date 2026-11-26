"""Slice 9 checks: stage2-gaps-g (header_checks/s09.d/, loaded after s09.py by spec/tools/header_check.py;
header_checks/README.md)."""

TAGS = ("enum __ptrace_setoptions", "enum __ptrace_eventcodes")


def units(ctx):
    """No spec clause beyond ptrace(2)'s PTRACE_SETOPTIONS options and PTRACE_EVENT_* stops (black-box probe of the
    reference, HEADERS-STAGE2-GAPS-G: glib2.0 uses PTRACE_O_TRACEFORK): where visibility.tsv shows them in
    <sys/ptrace.h>, each enumerator of TAGS has its enumerators.tsv value, is an enumerator of that tag (not a macro:
    `#ifdef` is false, as in the reference), and the tag is declared. Also <sys/fanotify.h>'s struct
    fanotify_event_metadata is complete (the kernel's, through <linux/fanotify.h>; liburing). Defects: a missing
    option (glib2.0 does not build), a wrong bit, a macro where the reference has an enumerator, the event record
    left incomplete or defined twice beside the kernel's."""
    enums = [r for r in ctx.rows("spec/abi/enumerators.tsv") if r["tag"] in TAGS]
    vis = {r["name"]: r["configs"] for r in ctx.vis if r["header"] == "sys/ptrace.h" and r["kind"] == "enumerator"}
    fan = {r["name"]: r["configs"] for r in ctx.vis if r["header"] == "sys/fanotify.h"}
    out = []
    for cid in ctx.ids:
        here = [r for r in enums if r["name"] in vis and cid in ctx.expand(vis[r["name"]])]
        if here:
            body = ["#include <sys/ptrace.h>"]
            for i, r in enumerate(here):
                body.append(f"#ifdef {r['name']}\n#error \"{r['name']} is a macro\"\n#endif")
                body.append(f"typedef char niji_pt{i}[{r['name']} == {r['value']} ? 1 : -1];")
                body.append(f"{r['tag']} niji_pv{i} = {r['name']};")
            out.append((cid, "ptrace option and event enumerators", "\n".join(body) + "\n"))
        if "fanotify_event_metadata" in fan and cid in ctx.expand(fan["fanotify_event_metadata"]):
            out.append((cid, "fanotify event record", "#include <sys/fanotify.h>\n"
                        "typedef char niji_fm[sizeof (struct fanotify_event_metadata) == 24 ? 1 : -1];\n"
                        "int niji_fp (const struct fanotify_event_metadata *m) { return m->fd + (int) m->pid; }\n"))
    return out
