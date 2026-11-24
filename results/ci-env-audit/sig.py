#!/usr/bin/env python3
# Summarise CI gate logs: one line per log with rc, wall, and the failure signature.
import glob, os, re, sys, datetime
n = int(sys.argv[1]) if len(sys.argv) > 1 else 60
logs = [f for f in glob.glob(os.path.expanduser('~/logs/ci-gate-*.log')) if not f.endswith('.affected.log')]
logs.sort(key=os.path.getmtime, reverse=True)
for f in logs[:n]:
    t = datetime.datetime.utcfromtimestamp(os.path.getmtime(f)).strftime('%m-%dT%H:%M')
    rc = None; tier = ''; total = ''; sig = []
    try:
        lines = open(f, errors='replace').read().splitlines()
    except Exception as e:
        print(os.path.basename(f), 'unreadable', e); continue
    for i, l in enumerate(lines):
        m = re.search(r'oom_kill=\d+ rc=(\d+)', l)
        if m: rc = m.group(1)
        if l.startswith('tier:'): tier = l.split()[1]
        if l.startswith('step-total:'): total = l.split()[1]
        if l.startswith('FAILED') or l.startswith('RELEASE STEP') or re.match(r'^(error(\[|:)|BAD|lint:|manifest:|ERROR|.*No space left|.*cannot allocate)', l):
            if 'to rerun pass' in l or l.startswith('error: test run failed') or l.startswith('error: test failed'):
                continue
            sig.append(l[:120])
    base = os.path.basename(f)[len('ci-gate-'):-4]
    print(f'{base} {t} rc={rc} tier={tier} total={total} | ' + ' ; '.join(sig[:6]))
