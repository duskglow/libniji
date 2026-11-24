#!/usr/bin/env python3
# Group red CI gate logs (newest N) by failure signature: count, first and last time, log ids.
import glob, os, re, sys, datetime, collections
n = int(sys.argv[1]) if len(sys.argv) > 1 else 300
logs = [f for f in glob.glob(os.path.expanduser('~/logs/ci-gate-*.log')) if not f.endswith('.affected.log')]
logs.sort(key=os.path.getmtime, reverse=True)
groups = collections.OrderedDict()
nred = ngreen = 0
for f in logs[:n]:
    t = datetime.datetime.fromtimestamp(os.path.getmtime(f), datetime.timezone.utc).strftime('%m-%dT%H:%M')
    txt = open(f, errors='replace').read()
    m = re.findall(r'oom_kill=\d+ rc=(\d+)', txt)
    rc = m[-1] if m else None
    if rc == '0':
        ngreen += 1; continue
    nred += 1
    lid = os.path.basename(f)[8:-4]
    aff = f[:-4] + '.affected.log'
    atxt = open(aff, errors='replace').read() if os.path.exists(aff) else ''
    sig = None
    for l in txt.splitlines():
        if l.startswith('FAILED:') or l.startswith('xtask affected: a generator precheck failed') or l.startswith('RELEASE STEP FAILED'):
            sig = l[:110]; break
        if l.startswith('error[') or l.startswith('error: could not compile'):
            sig = 'compile: ' + l[:90]; break
    if sig is None:
        for l in atxt.splitlines():
            if l.startswith('xtask test:') and ('lack' in l or 'stale' in l or 'missing' in l):
                sig = 'xtask test: ' + l[12:100]; break
            if l.startswith('STEP FAILED'):
                sig = l[:110]
    if sig is None:
        sig = 'rc=%s no-signature' % rc
    g = groups.setdefault(sig, [])
    g.append((t, lid))
print(f'logs={min(n, len(logs))} red={nred} green={ngreen} oldest={datetime.datetime.fromtimestamp(os.path.getmtime(logs[min(n, len(logs)) - 1]), datetime.timezone.utc).strftime("%m-%dT%H:%M")}')
for sig, g in groups.items():
    print(f'{len(g):3d} {g[-1][0]}..{g[0][0]} {sig}')
    print('      ' + ' '.join(x[1].split('-')[0] for x in g))
