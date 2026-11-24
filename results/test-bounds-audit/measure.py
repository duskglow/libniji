#!/usr/bin/env python3
"""TEST-BOUNDS-AUDIT sensitivity sweep (runs on a build host, in a role dir with fresh products).

measure.py <role dir> <out dir> <label> [--perf] [--glibc]

For the static probe (oracle/page_faults.c against target/niji/libc.a) linked with a start file padded in
several regions (as hot_text_products.rs's phase tests do, plus a full 4 KiB-step sweep of the cold pad over
one 64 KiB window), and for the dynamic probe (staged ld.so + libc.so.6), runs every mode with the test's
discipline (no ASLR, argv0 page_faults, cwd /, env LANG only, warm-up + 5, min) from two staging file
systems (/tmp and /dev/shm, equal-length paths). Writes counts.tsv (label fs config mode min all-runs) and,
with --perf, faults.tsv (label fs config mode region key) from perf page-fault addresses.
With --glibc also the same probe built by the host compiler and C library (static and dynamic)."""
import ctypes, hashlib, os, shutil, subprocess, sys

role, out, label = sys.argv[1], sys.argv[2], sys.argv[3]
PERF, GLIBC = '--perf' in sys.argv, '--glibc' in sys.argv
os.makedirs(out, exist_ok=True)
P = os.path.join(role, 'target/niji')
SRC = os.path.join(role, 'crates/core/tests/oracle/page_faults.c')
work = os.path.join(out, 'build-' + label)
shutil.rmtree(work, ignore_errors=True)
os.makedirs(work)
MODES = ['true', 'locale', 'tool', 'fork', 'localtime', 'localtime_mktime', 'float', 'strings', 'mktime', 'rebind']

libc = ctypes.CDLL(None, use_errno=True)
cur = libc.personality(0xffffffff)
libc.personality(cur | 0x0040000)  # ADDR_NO_RANDOMIZE, inherited by every child

def sh(args, **kw):
    r = subprocess.run(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE, **kw)
    if r.returncode != 0:
        raise SystemExit(f'failed: {args}\n{r.stderr.decode(errors="replace")[-2000:]}')
    return r.stdout.decode(errors='replace')

with open(os.path.join(out, f'products-{label}.sha'), 'w') as f:
    for p in ['libc.a', 'libc.so.6', 'ld-linux-x86-64.so.2', 'niji-crt1.o', 'libm.a']:
        fp = os.path.join(P, p)
        if os.path.exists(fp):
            f.write(f'{hashlib.sha256(open(fp, "rb").read()).hexdigest()}  {p}\n')

def padded(region, pad, d):
    body = {
        'data': f'.section .data.niji_pad,"aw",@progbits\n.fill {pad},1,0\n',
        'pre': f'.section .text.unlikely.niji_pad,"ax",@progbits\n.fill {pad},1,0xcc\n',
        'lbss': f'.section .lbss.niji_pad,"aw",@nobits\n.zero {pad}\n',
        'relro': f'.section .data.rel.ro.local.niji_pad,"aw",@progbits\n.fill {pad},1,0\n',
    }.get(region, f'.text\n.fill {pad},1,0xcc\n')
    open(os.path.join(d, 'pad.s'), 'w').write(body)
    sh(['cc', '-c', os.path.join(d, 'pad.s'), '-o', os.path.join(d, 'pad.o')])
    sh(['ld', '-r', '-o', os.path.join(d, 'crt1pad.o'), os.path.join(P, 'niji-crt1.o'), os.path.join(d, 'pad.o')])
    return os.path.join(d, 'crt1pad.o')

def link_static(crt1, exe):
    sh(['cc', '-std=gnu17', '-O1', '-static', '-nostdlib', '-fno-builtin', f'-Wl,-Map={exe}.map', crt1, SRC,
        os.path.join(P, 'libc.a'), '-lgcc', '-o', exe])

CONFIGS = [('none', 0)] + [('cold', 0x800)] + [('cold', k * 0x1000) for k in range(1, 16)] + \
          [('pre', 0x800), ('pre', 0x2000), ('pre', 0x8000), ('data', 0x800), ('data', 0x1000), ('data', 0x2000),
           ('data', 0x3000), ('lbss', 0x800), ('lbss', 0x2000), ('relro', 0x4d0), ('relro', 0xc18)]
exes = []
for region, pad in CONFIGS:
    d = os.path.join(work, f'{region}-{pad:x}')
    os.makedirs(d)
    crt1 = os.path.join(P, 'niji-crt1.o') if pad == 0 else padded(region, pad, d)
    exe = os.path.join(d, 'page_faults')
    link_static(crt1, exe)
    exes.append((f'{region}-{pad:x}', exe, MODES, {}))

# dynamic probe of the products (dyn_startup_faults), staged per file system below
def gccf(f):
    return sh(['cc', f'-print-file-name={f}']).strip()

if GLIBC:
    g = os.path.join(work, 'glibc')
    os.makedirs(g)
    sh(['cc', '-std=gnu17', '-O1', '-static', SRC, '-o', os.path.join(g, 'pf_static')])
    sh(['cc', '-std=gnu17', '-O1', '-fPIE', '-pie', SRC, '-o', os.path.join(g, 'pf_dyn')])
    exes.append(('glibc-static', os.path.join(g, 'pf_static'), MODES, {}))
    exes.append(('glibc-dyn', os.path.join(g, 'pf_dyn'), MODES, {}))

FS = [('ext4', '/tmp/niji-tba-xxxxxx'),('tmpfs', '/dev/shm/niji-tba-xx')]
assert len(FS[0][1]) == len(FS[1][1])

def run(path, mode, env, perfdata=None):
    argv = ['page_faults', 'relay', mode]
    if perfdata:
        # perf itself must not see the program's LD_LIBRARY_PATH: /usr/bin/env sets it for the program only
        extra = [f'{k}={v}' for k, v in env.items() if k != 'LANG']
        args = ['/usr/bin/perf', 'record', '-q', '-e', 'page-faults', '-c', '1', '-d', '-o', perfdata, '--', '/usr/bin/env', *extra, path, 'relay', mode]
        r = subprocess.run(args, cwd='/', env={'LANG': env['LANG']}, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        if not os.path.exists(perfdata):
            print('PERF-FAILED', path, mode, r.returncode, r.stderr.decode(errors='replace')[-600:], flush=True)
    else:
        r = subprocess.run(argv, executable=path, cwd='/', env=env, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    t = r.stderr.decode(errors='replace')
    for l in t.splitlines():
        if l.startswith('minflt '):
            return int(l.split()[1])
    return -1

def regions(exe_path):
    """(start, end, name) of the exe's allocated sections."""
    secs = []
    for l in sh(['readelf', '-SW', exe_path]).splitlines():
        if ']' not in l:
            continue
        f = l.split(']', 1)[1].split()
        if len(f) > 5 and f[2] != '0000000000000000':
            try:
                a, z = int(f[2], 16), int(f[4], 16)
            except ValueError:
                continue
            secs.append((a, a + max(z, 1), f[0]))
    return secs

cf = open(os.path.join(out, 'counts.tsv'), 'a')
ff = open(os.path.join(out, 'faults.tsv'), 'a') if PERF else None
for fsname, sdir in FS:
    shutil.rmtree(sdir, ignore_errors=True)
    os.makedirs(sdir)
    # dynamic products probe, staged on this file system
    lib = os.path.join(sdir, 'lib')
    os.makedirs(lib)
    for f in ['ld-linux-x86-64.so.2', 'libc.so.6']:
        sh(['cp', '--', os.path.join(P, f), os.path.join(lib, f)])
    dexe = os.path.join(lib, 'page_faults_dyn')
    sh(['cc', '-std=gnu17', '-O1', '-nostdlib', '-fno-builtin', '-fPIE', '-pie',
        f'-Wl,--dynamic-linker={lib}/ld-linux-x86-64.so.2', os.path.join(P, 'niji-crt1.o'), gccf('crtbeginS.o'), SRC,
        os.path.join(P, 'libc_nonshared.a'), os.path.join(lib, 'libc.so.6'), os.path.join(lib, 'ld-linux-x86-64.so.2'),
        '-lgcc', gccf('crtendS.o'), '-o', dexe])
    todo = exes + [('niji-dyn', dexe, ['true', 'locale', 'tool', 'fork'], {'LD_LIBRARY_PATH': lib})]
    for cfg, exe, modes, extra in todo:
        run_path = os.path.join(sdir, 'p')
        sh(['cp', '--', exe, run_path])
        env = {'LANG': 'en_US.UTF-8', **extra}
        for mode in modes:
            seen = [run(run_path, mode, env) for _ in range(6)]
            cf.write(f'{label}\t{fsname}\t{cfg}\t{mode}\t{min(seen[1:])}\t{",".join(map(str, seen))}\n')
            if ff and mode in ('true', 'tool', 'locale', 'fork') and (cfg.startswith(('none', 'cold', 'data', 'glibc', 'niji-dyn'))):
                pd = os.path.join(sdir, 'perf.data')
                run(run_path, mode, env, pd)
                if not os.path.exists(pd):
                    continue
                txt = subprocess.run(['/usr/bin/perf', 'script', '-i', pd, '-F', 'pid,addr', '--show-mmap-events'], stdout=subprocess.PIPE,
                                     stderr=subprocess.DEVNULL).stdout.decode(errors='replace')
                secs = regions(run_path) if not cfg.endswith('dyn') else []
                maps, keys = [], []
                for l in txt.splitlines():
                    if 'PERF_RECORD_MMAP' in l:
                        # ... [0xstart(0xlen) @ 0xpgoff ...]: r-xp /path
                        try:
                            br = l.split('[', 1)[1]
                            st = int(br.split('(')[0], 16)
                            ln = int(br.split('(')[1].split(')')[0], 16)
                            perm_path = l.rsplit(']:', 1)[1].split()
                            maps.append((st, st + ln, os.path.basename(perm_path[-1]) + ':' + perm_path[0]))
                        except (IndexError, ValueError):
                            pass
                        continue
                    f = l.split()
                    if len(f) < 2:
                        continue
                    try:
                        a = int(f[-1], 16)
                    except ValueError:
                        continue
                    name = next((n for s, e, n in secs if s <= a < e), None) or next((n for s, e, n in maps if s <= a < e), None)
                    if name is None:
                        name = 'stack' if a >= 0x7ff000000000 else ('anon' if a >= 0x7f0000000000 else 'other')
                    keys.append((f[0], name, a))
                for pid, name, a in keys:
                    ff.write(f'{label}\t{fsname}\t{cfg}\t{mode}\t{pid}\t{name}\t{a:#x}\n')
                os.unlink(pd)
        cf.flush()
    shutil.rmtree(sdir, ignore_errors=True)
print('done', label)
