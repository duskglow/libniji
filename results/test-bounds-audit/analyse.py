#!/usr/bin/env python3
"""Summarise measure.py output: analyse.py <out dir> [label...]
1. per label/fs/mode: the count at config none, the distinct values over the cold sweep and the other pads,
   and the configs that differ from none;
2. fs sensitivity: configs/modes whose count differs between ext4 and tmpfs;
3. perf attribution: per label/fs/config/mode (counted child only = the pid with the most samples after the
   first), faults per region, and for configs that differ from none the regions whose fault count differs."""
import collections, os, sys

out = sys.argv[1]
labels = sys.argv[2:]
C = collections.OrderedDict()
for l in open(os.path.join(out, 'counts.tsv')):
    lab, fs, cfg, mode, mn, runs = l.rstrip('\n').split('\t')
    if labels and lab not in labels:
        continue
    C[(lab, fs, cfg, mode)] = (int(mn), runs)

print('## 1. counts per label / fs / mode (none; values over cold 4K sweep; other pads; differing configs)')
groups = collections.OrderedDict()
for (lab, fs, cfg, mode), (n, runs) in C.items():
    groups.setdefault((lab, fs, mode), []).append((cfg, n, runs))
nonident = []
for (lab, fs, mode), rows in groups.items():
    d = dict((c, n) for c, n, _ in rows)
    for c, n, runs in rows:
        vals = set(runs.split(',')[1:])
        if len(vals) > 1:
            nonident.append(f'{lab} {fs} {c} {mode}: runs {runs}')
    base = d.get('none-0')
    cold = sorted({n for c, n in d.items() if c.startswith('cold')})
    other = sorted({n for c, n in d.items() if c.startswith(('data', 'lbss', 'relro'))})
    pre = sorted({n for c, n in d.items() if c.startswith('pre')})
    diff = [f'{c}={n}' for c, n in d.items() if base is not None and n != base and not c.startswith(('pre', 'glibc', 'niji-dyn'))]
    single = [c for c in d if c.startswith(('glibc', 'niji-dyn'))]
    if single:
        print(f'{lab:8} {fs:5} {mode:17} ' + ' '.join(f'{c}={d[c]}' for c in single))
    if base is not None:
        print(f'{lab:8} {fs:5} {mode:17} none={base} cold={cold} data/lbss/relro={other} pre={pre} differ: {" ".join(diff) or "-"}')
print('## runs not identical (warm-up excluded):', len(nonident))
for x in nonident[:20]:
    print('  ', x)

print('## 2. ext4 vs tmpfs')
nfs = 0
for (lab, fs, cfg, mode), (n, _) in C.items():
    if fs != 'ext4':
        continue
    m = C.get((lab, 'tmpfs', cfg, mode))
    if m and m[0] != n:
        nfs += 1
        print(f'  {lab} {cfg} {mode}: ext4 {n} tmpfs {m[0]}')
print(f'  {nfs} differing of {sum(1 for k in C if k[1] == "ext4")}')

fp = os.path.join(out, 'faults.tsv')
if os.path.exists(fp):
    print('## 3. perf attribution (counted process)')
    F = collections.defaultdict(list)
    for l in open(fp):
        lab, fs, cfg, mode, pid, region, addr = l.rstrip('\n').split('\t')
        if labels and lab not in labels:
            continue
        F[(lab, fs, cfg, mode)].append((pid, region, int(addr, 16)))
    def counted(rows):
        pids = list(dict.fromkeys(p for p, _, _ in rows))
        # the relay parent comes first; the counted child is the last exec'd pid with samples
        pid = pids[-1] if len(pids) > 1 else pids[0]
        return [(r, a) for p, r, a in rows if p == pid]
    def summary(rows):
        c = collections.Counter()
        for r, a in rows:
            c[r] += 1
        return c
    for key in sorted(F):
        lab, fs, cfg, mode = key
        if not (cfg in ('none-0',) or cfg.startswith(('glibc', 'niji-dyn'))):
            continue
        s = summary(counted(F[key]))
        print(f'  {lab} {fs} {cfg} {mode}: total {sum(s.values())}: ' + ', '.join(f'{r} {n}' for r, n in s.most_common()))
    print('## 3b. configs whose count differs from none: region deltas and the moved addresses')
    for (lab, fs, cfg, mode), (n, _) in C.items():
        b = C.get((lab, fs, 'none-0', mode))
        if not b or n == b[0] or cfg.startswith('pre') or (lab, fs, cfg, mode) not in F:
            continue
        s0, s1 = summary(counted(F[(lab, fs, 'none-0', mode)])), summary(counted(F[(lab, fs, cfg, mode)]))
        delta = {r: s1[r] - s0[r] for r in set(s0) | set(s1) if s1[r] != s0[r]}
        rows1 = counted(F[(lab, fs, cfg, mode)])
        moved = [f'{r}@{a:#x}' for r, a in rows1 if r in delta][:12]
        print(f'  {lab} {fs} {cfg} {mode}: {b[0]} -> {n}; regions {delta}; {" ".join(moved)}')
