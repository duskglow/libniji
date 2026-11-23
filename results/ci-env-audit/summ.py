#!/usr/bin/env python3
# Summarise an strace -f file-access trace: every path outside the role tree and the toolchain home,
# grouped by prefix, with counts of successful and failed (ENOENT etc.) accesses; and every program exec'd.
import collections, os, re, sys
role, trace = sys.argv[1], sys.argv[2]
home = os.path.expanduser('~')
skip_prefixes = (role + '/', home + '/.cargo/', home + '/.rustup/')
call_re = re.compile(r'^\d+\s+(\w+)\((.*)\)\s+=\s+(-?\d+|\?)(?:\s+(\w+))?')
path_re = re.compile(r'"((?:[^"\\]|\\.)*)"')
ok = collections.Counter(); bad = collections.Counter(); badkind = collections.defaultdict(collections.Counter)
execs = collections.Counter(); exec_missing = collections.Counter()
writes = collections.Counter()
def group(p):
    parts = p.split('/')
    if p.startswith('/proc/') and len(parts) > 2 and (parts[2].isdigit() or parts[2] in ('self', 'thread-self')):
        return '/proc/<pid>/' + '/'.join(parts[3:4])
    if p.startswith('/tmp/') or p.startswith('/dev/shm/'):
        return '/'.join(parts[:2]) + '/<scratch>'
    if p.startswith(home + '/'):
        return '~/' + '/'.join(parts[3:5])
    depth = 4 if p.startswith(('/usr/lib/', '/usr/share/', '/usr/include/', '/usr/lib64/', '/sys/')) else 3
    return '/'.join(parts[:depth])
with open(trace, errors='replace') as f:
    for line in f:
        m = call_re.match(line)
        if not m:
            continue
        name, args, ret, err = m.groups()
        ps = path_re.findall(args)
        if not ps:
            continue
        p = ps[0]
        if name in ('openat', 'newfstatat', 'faccessat', 'faccessat2', 'readlinkat', 'mkdirat', 'unlinkat', 'statx', 'openat2') and not p.startswith('/'):
            continue  # relative to a dirfd: almost always inside the role
        if name == 'execve':
            prog = os.path.basename(p)
            (execs if ret == '0' else exec_missing)[prog] += 1
            if ret == '0':
                ok['EXEC ' + p] += 1
            continue
        if not p.startswith('/') or p.startswith(skip_prefixes) or p == role:
            continue
        g = group(p)
        if ret.startswith('-'):
            bad[g] += 1; badkind[g][err or '?'] += 1
        else:
            ok[g] += 1
            if name in ('mkdir', 'mkdirat', 'unlink', 'unlinkat', 'rename', 'renameat', 'renameat2') or ('O_WRONLY' in args or 'O_RDWR' in args or 'O_CREAT' in args):
                writes[g] += 1
print('# programs executed (successful execve), count')
print(' '.join(f'{k}:{v}' for k, v in sorted(execs.items(), key=lambda x: -x[1])))
print('# programs looked up but absent (failed execve, PATH search included), count')
print(' '.join(f'{k}:{v}' for k, v in sorted(exec_missing.items(), key=lambda x: -x[1])[:60]))
print('# host paths read (outside role, ~/.cargo, ~/.rustup): group ok fail fail-kinds writes')
keys = set(k for k in ok if not k.startswith('EXEC ')) | set(bad)
for k in sorted(keys, key=lambda k: -(ok[k] + bad[k])):
    kinds = ','.join(f'{e}:{c}' for e, c in badkind[k].most_common(3))
    print(f'{k}\t{ok[k]}\t{bad[k]}\t{kinds}\t{writes[k]}')
