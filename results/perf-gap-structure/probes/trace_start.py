# gdb -q -nx -batch -x trace_start.py --args <libniji program> : exact user-instruction split of a whole process
# (PERF-GAP-STRUCTURE-AUDIT). Single-steps from the first instruction (starti) to exit; per function: exclusive
# steps and inclusive steps (a shadow call stack from call/ret). libniji programs only (never the reference).
# A rep-prefixed string instruction steps once per iteration (perf counts it once): noted in the report.
import collections
import os

import gdb

gdb.execute("set pagination off")
gdb.execute("set confirm off")
gdb.execute("set startup-with-shell off")
gdb.execute("unset environment LINES")
gdb.execute("unset environment COLUMNS")
gdb.execute("starti", to_string=True)
inf = gdb.selected_inferior()
limit = int(os.environ.get("TRACE_MAX", "4000000"))
excl = collections.Counter()
incl = collections.Counter()
byobj = collections.Counter()
names = {}
stack = []
steps = 0
prev_call = prev_ret = False


def fn(pc):
    s = names.get(pc)
    if s is None:
        t = gdb.execute("info symbol %d" % pc, to_string=True).strip()
        if t.startswith("No symbol"):
            f, o = "?", "?"
        else:
            f = t.split(" + ")[0].split(" in section ")[0]
            o = os.path.basename(t.split(" of ")[1]) if " of " in t else "program"
        s = names[pc] = (f, o)
    return s


while steps < limit:
    try:
        pc = int(gdb.selected_frame().pc())
    except gdb.error:
        break
    f, o = fn(pc)
    if prev_call:
        stack.append(f)
    elif prev_ret and stack:
        stack.pop()
    excl[f] += 1
    byobj[o] += 1
    for nm in set(stack):
        incl[nm] += 1
    try:
        b = inf.read_memory(pc, 3).tobytes()
    except gdb.error:
        b = b"\0\0\0"
    i = 1 if 0x40 <= b[0] <= 0x4F else 0
    prev_call = b[i] == 0xE8 or (b[i] == 0xFF and (b[i + 1] >> 3) & 7 == 2)
    prev_ret = b[0] in (0xC3, 0xC2) or (b[0] == 0xF3 and b[1] == 0xC3)
    steps += 1
    try:
        gdb.execute("stepi", to_string=True)
    except gdb.error:
        break
    if not inf.is_valid() or inf.pid == 0:
        break

print("TRACE steps %d (limit %d), unique pcs %d" % (steps, limit, len(names)))
print("TRACE by object: " + ", ".join("%s %d" % kv for kv in byobj.most_common()))
print("TRACE exclusive (top 45):")
for f, c in excl.most_common(45):
    print("  %7d %5.1f%%  %s" % (c, 100.0 * c / steps, f[:150]))
print("TRACE inclusive (top 45, from the shadow call stack):")
for f, c in incl.most_common(45):
    print("  %7d %5.1f%%  %s" % (c, 100.0 * c / steps, f[:150]))
