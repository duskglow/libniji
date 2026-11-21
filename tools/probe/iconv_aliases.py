#!/usr/bin/env python3
"""The listed charset names that stand for a charmap, from a black-box probe (ICONV-REST).

usage: iconv_aliases.py FINGERPRINT CHARMAP_DIR OUT_DIR

Run on the reference system. FINGERPRINT is tools/probe/iconv_fingerprint.c built against the
reference C library; CHARMAP_DIR its installed charmaps (/usr/share/i18n/charmaps). Reads the
names the reference's `iconv -l` lists, the charmaps (each file's name, `<code_set_name>` and
`% alias` names; also spec/encoding/charmaps.tsv's generated charmaps and their aliases) and the
built-in names (csys/util/iconv-names.h), and fingerprints every one of these names on the
reference. A charmap is the converter of any of its names the reference opens. A listed name
that is not a built-in name or a charmap file name stands for the charmap whose converter has the
same fingerprint; of several, one that carries the name itself (a generated charmap first), then
one with a name of the same letters and digits, then one both charmap sets have, then the
installed one, then the first by name. Writes, in OUT_DIR:
  iconv-aliases.tsv  the rows (name<TAB>charmap file name, both in matching form, sorted by name)
  iconv-aliases.h    the same rows for the `iconv` command (csys/util/iconv-aliases.h)
  unsupported.txt    listed names with no built-in or charmap of the same conversion
  report.txt         listed names that convert as a built-in without being one of its names,
                     and names left out (a `/` in the name)
Python 3 standard library only; run from the repository root.
"""
import gzip
import os
import re
import subprocess
import sys

SIG = set(b'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789,-./:_')
HEAD = ('# Listed charset names that stand for a charmap (spec/encoding/iconv-names.md, rule 5):\n'
        '# name<TAB>charmap file name, both in matching form, sorted by name. Written by\n'
        '# tools/probe/iconv_aliases.py from a black-box probe of the reference (same conversion by\n'
        '# fingerprint, tools/probe/iconv_fingerprint.c). Read by crates/core/src/iconv/aliases.rs.\n')


def form(n):
    return bytes(b for b in n.encode() if b in SIG).decode().upper()


def alnum(n):
    return ''.join(c for c in n.upper() if c.isalnum())


def header_names(path):
    opener = gzip.open if path.endswith('.gz') else open
    names = []
    with opener(path, 'rt', encoding='latin-1') as f:
        for line in f:
            if line.startswith('CHARMAP'):
                break
            m = re.match(r'<code_set_name>\s+(\S+)', line) or re.match(r'%\s*alias\s+(\S+)', line)
            if m:
                names.append(m.group(1))
    return names


def main():
    fpbin, cmdir, out = sys.argv[1:4]
    listed = [l.strip().rstrip('/') for l in subprocess.run(['iconv', '-l'], capture_output=True, text=True, check=True).stdout.splitlines() if l.strip()]
    hdr = open('csys/util/iconv-names.h').read()
    builtins = re.findall(r'"([^"]+)"', hdr.split('BUILTIN_NAMES[] = {', 1)[1].split('};', 1)[0])
    distro, niji = {}, {}
    for e in sorted(os.listdir(cmdir)):
        stem = e[:-3] if e.endswith('.gz') else e
        if stem and not stem.startswith('.'):
            distro[stem] = [stem] + header_names(os.path.join(cmdir, e))
    for line in open('spec/encoding/charmaps.tsv'):
        f = line.rstrip('\n').split('\t')
        if line.startswith('#') or len(f) < 3:
            continue
        niji[f[0]] = [f[0]] + (f[3].split() if len(f) > 3 else [])
    allnames = sorted(set(listed) | set(builtins) | {n for d in (distro, niji) for ns in d.values() for n in ns})
    run = subprocess.run([fpbin], input='\n'.join(allnames) + '\n', capture_output=True, text=True, check=True)
    fp = {n: h for n, h in (l.split('\t') for l in run.stdout.splitlines())}
    charmaps = {}
    for d in (distro, niji):
        for stem, ns in d.items():
            c = charmaps.setdefault(form(stem), {'names': set(), 'gen': set(), 'fps': set(), 'sets': 0})
            c['names'] |= set(ns)
            if d is niji:
                c['gen'] |= set(ns)
            c['fps'] |= {fp[n] for n in ns if fp.get(n, 'NOOPEN') != 'NOOPEN'}
            c['sets'] |= 1 if d is distro else 2
    bforms = {form(b) for b in builtins}
    bfps = {fp[b]: b for b in builtins if fp.get(b, 'NOOPEN') != 'NOOPEN'}
    rows, none, report = {}, [], []
    for n in listed:
        f, h = form(n), fp.get(n, 'NOOPEN')
        if f in bforms or f in charmaps:
            continue
        if h in bfps:
            report.append(f'converts as built-in {bfps[h]}: {n}')
            continue
        if '/' in f:
            report.append(f'left out (a `/` in the name): {n}')
            continue
        cands = sorted(s for s, c in charmaps.items() if h in c['fps'])
        if not cands:
            none.append(n)
            continue
        gen = [s for s in cands if any(form(m) == f for m in charmaps[s]['gen'])]
        own = gen or [s for s in cands if any(form(m) == f for m in charmaps[s]['names'])]
        same = [s for s in cands if any(alnum(m) == alnum(n) for m in charmaps[s]['names'])]
        both = [s for s in cands if charmaps[s]['sets'] == 3]
        inst = [s for s in cands if charmaps[s]['sets'] & 1]
        rows[f] = (own or same or both or inst or cands)[0]
    out_rows = sorted(rows.items(), key=lambda kv: kv[0].encode())
    with open(os.path.join(out, 'iconv-aliases.tsv'), 'w') as fh:
        fh.write(HEAD + ''.join(f'{n}\t{t}\n' for n, t in out_rows))
    with open(os.path.join(out, 'iconv-aliases.h'), 'w') as fh:
        fh.write('/* The listed charset names that stand for a charmap (crates/core/src/iconv/aliases.rs, from\n'
                 ' * spec/encoding/iconv-aliases.tsv): `iconv -l` lists a row when its charmap is installed. Written by\n'
                 ' * tools/probe/iconv_aliases.py; the core test `iconv_listing_aliases_match_the_library`\n'
                 ' * (crates/core/tests/iconv_cmd_names.rs) fails when these rows and the library\'s differ. */\n'
                 'static const char *const ALIAS_ROWS[][2] = {\n'
                 + ''.join(f'    {{"{n}", "{t}"}},\n' for n, t in out_rows) + '};\n')
    with open(os.path.join(out, 'unsupported.txt'), 'w') as fh:
        fh.write(''.join(n + '\n' for n in none))
    with open(os.path.join(out, 'report.txt'), 'w') as fh:
        fh.write(''.join(r + '\n' for r in report))
    print(f'listed {len(listed)}, rows {len(out_rows)}, unsupported {len(none)}, report {len(report)}')


main()
