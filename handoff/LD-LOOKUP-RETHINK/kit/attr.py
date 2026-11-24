#!/usr/bin/env python3
"""LD-LOOKUP-RETHINK: attribute an sstep trace (libniji's own loader). usage: attr.py TRACE_PREFIX OUT.json
TRACE.seq holds (rip, rsp) pairs, TRACE.maps the maps at exit. Every executed address is symbolised in its mapped
ELF file: nm -S for function ranges (self and rsp-bounded inclusive counts), addr2line -a -f -i -C for the inline
chain from the line tables (innermost inlined function, its source line, outermost function).
JSON: total, obj{file: n}, self{fn}, incl{fn}, inl{innermost fn}, line{file:line}, pair{outer|innermost}."""
import array, bisect, collections, json, os, subprocess, sys

pre, outp = sys.argv[1], sys.argv[2]
symfile = dict(x.split("=", 1) for x in sys.argv[3:])  # basename=file with the same code and its symbols/lines
seq = array.array("Q")
with open(pre + ".seq", "rb") as f:
    seq.frombytes(f.read())
rips, rsps = seq[0::2], seq[1::2]
total = len(rips)

maps = []  # (start, end, path)
bases = {}
for line in open(pre + ".maps"):
    p = line.split()
    if len(p) < 6 or not p[5].startswith("/"):
        continue
    s, e = (int(x, 16) for x in p[0].split("-"))
    maps.append((s, e, p[5]))
    if int(p[2], 16) == 0:
        bases[p[5]] = min(bases.get(p[5], s), s)
maps.sort()
for path in list(bases):
    with open(path, "rb") as f:
        hdr = f.read(18)
    if hdr[16] == 2:  # ET_EXEC: absolute addresses
        bases[path] = 0
starts = [m[0] for m in maps]

def where(a):
    i = bisect.bisect_right(starts, a) - 1
    if i >= 0 and a < maps[i][1]:
        return maps[i][2]
    return None

count = collections.Counter(rips)
by_file = collections.defaultdict(list)
for a in count:
    by_file[where(a)].append(a)

def symbols(path):
    out = []
    for flags in (["-S", "-n", "-C", "--defined-only"], ["-D", "-S", "-n", "-C", "--defined-only"]):
        r = subprocess.run(["nm"] + flags + [path], capture_output=True, text=True)
        for line in r.stdout.splitlines():
            p = line.split(None, 3)
            if len(p) == 4 and p[2] in "TtWwi":
                out.append((int(p[0], 16), int(p[1], 16), p[3]))
        if out:
            break
    out.sort()
    return out

info = {}  # address -> (function by nm, innermost inline fn, line, outermost fn)
entry_name = {}  # absolute function entry -> name
obj = collections.Counter()
for path, addrs in by_file.items():
    short = os.path.basename(path) if path else "?"
    obj[short] += sum(count[a] for a in addrs)
    if not path:
        for a in addrs:
            info[a] = ("?", "?", "?", "?")
        continue
    base = bases.get(path, 0)
    path = symfile.get(short, path)
    syms = symbols(path)
    sstart = [s[0] for s in syms]
    for s, z, name in syms:
        entry_name[s + base] = short + ":" + name
    rel = sorted(a - base for a in addrs)
    chains = {}
    r = subprocess.run(["addr2line", "-a", "-f", "-i", "-C", "-e", path], input="".join("%x\n" % a for a in rel),
                       capture_output=True, text=True)
    cur = None
    lines = r.stdout.splitlines()
    i = 0
    while i < len(lines):
        if lines[i].startswith("0x"):
            cur = int(lines[i], 16)
            chains[cur] = []
            i += 1
            continue
        fn = lines[i]
        loc = lines[i + 1] if i + 1 < len(lines) else "?"
        chains.setdefault(cur, []).append((fn, loc))
        i += 2
    for a in rel:
        j = bisect.bisect_right(sstart, a) - 1
        fn = short + ":" + (syms[j][2] if j >= 0 and a < syms[j][0] + max(syms[j][1], 1) else "?")
        ch = chains.get(a) or [("?", "?")]
        loc = ch[0][1]
        if "/crates/" in loc:
            loc = "crates/" + loc.split("/crates/", 1)[1]
        loc = loc.split(" (discriminator")[0]
        info[a + base] = (fn, ch[0][0], loc, ch[-1][0])

self_c = collections.Counter()
inl = collections.Counter()
line_c = collections.Counter()
pair = collections.Counter()
for a, c in count.items():
    fn, inner, loc, outer = info[a]
    self_c[fn] += c
    inl[inner] += c
    line_c[loc] += c
    pair[outer + " | " + inner] += c

incl = collections.Counter()
stack = []  # (name, rsp at entry, index at entry)
active = collections.Counter()
for i in range(total):
    sp = rsps[i]
    while stack and sp > stack[-1][1]:
        name, _, at = stack.pop()
        active[name] -= 1
        if active[name] == 0:
            incl[name] += i - at
    name = entry_name.get(rips[i])
    if name is not None:
        stack.append((name, sp, i))
        active[name] += 1
while stack:
    name, _, at = stack.pop()
    active[name] -= 1
    if active[name] == 0:
        incl[name] += total - at

json.dump({"total": total, "obj": obj, "self": self_c, "incl": incl, "inl": inl, "line": line_c, "pair": pair},
          open(outp, "w"))
print("attr %s: %d instructions, %d addresses, objects %s" % (pre, total, len(count), dict(obj.most_common(6))))
