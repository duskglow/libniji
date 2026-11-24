#!/usr/bin/env python3
# Per CI gate log (newest N): time, rc, and the step seconds of the build steps, plus whether target/niji was cleared.
import glob, os, re, sys, datetime
n = int(sys.argv[1]) if len(sys.argv) > 1 else 40
want = ['clippy-workspace', 'so/total', 'test/products', 'test/tests-nextest', 'products']
logs = [f for f in glob.glob(os.path.expanduser('~/logs/ci-gate-*.log')) if not f.endswith('.affected.log')]
logs.sort(key=os.path.getmtime, reverse=True)
print('id time rc ' + ' '.join(want) + ' total cleared compiling_lines')
for f in logs[:n]:
    t = datetime.datetime.fromtimestamp(os.path.getmtime(f), datetime.timezone.utc).strftime('%m-%dT%H:%M')
    txt = open(f, errors='replace').read()
    aff = f[:-4] + '.affected.log'
    atxt = open(aff, errors='replace').read() if os.path.exists(aff) else ''
    both = txt + '\n' + atxt
    m = re.findall(r'oom_kill=\d+ rc=(\d+)', txt)
    rc = m[-1] if m else '-'
    vals = []
    for w in want:
        mm = re.findall(r'^step(?:-summary)?: ' + re.escape(w) + r' ([0-9.]+)s', both, re.M)
        vals.append(mm[-1] if mm else '-')
    tot = re.findall(r'^step-total: ([0-9.]+)s', both, re.M)
    cleared = 'yes' if re.search(r'clear|removed target/niji|rm -rf .*target/niji', both[:20000]) else '-'
    comp = len(re.findall(r'^\s*Compiling ', both, re.M))
    print(os.path.basename(f)[8:17], t, rc, ' '.join(vals), tot[-1] if tot else '-', cleared, comp)
