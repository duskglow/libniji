#!/usr/bin/env python3
"""Split crates/boundary/src/ld.rs into ld/<module>.rs by moving whole top-level items.

Usage: split.py <repo> <stage>   (stage k = the first k modules of ORDER are moved; reads the
pristine ld.rs from <repo>/../.scratch copy given by LD_ORIG). Writes ld.rs and ld/<m>.rs."""
import os, re, sys

ORIG = os.environ['LD_ORIG']
repo, stage = sys.argv[1], int(sys.argv[2])
SRC = os.path.join(repo, 'crates/boundary/src')

ITEM = re.compile(r'^(?:pub(?:\([a-z]+\))?\s+)?(?:unsafe\s+)?(?:extern\s+"C"\s+)?'
                  r'(fn|struct|enum|static|const|type|mod|impl|trait|use|macro_rules!)\s*(?:<[^>]*>\s*)?'
                  r'([A-Za-z_][A-Za-z0-9_]*)?')


def strip_code(l):
    code = re.sub(r'//.*', '', l)
    code = re.sub(r"b?'(\\.|[^'\\])'", "''", code)
    return re.sub(r'b?"(\\.|[^"\\])*"', '""', code)


def chunks(lines):
    """[(first_line_index, lines, blank_before)] split at blank lines at depth 0, and at a `///`
    line that follows a `;`-terminated line at depth 0."""
    out, cur, depth, blank = [], [], 0, True
    start = 0
    for i, l in enumerate(lines):
        s = l.strip()
        if depth == 0 and s == '':
            if cur:
                out.append([start, cur, blank]); cur = []
            blank = True
            continue
        if depth == 0 and cur and s.startswith('///') and cur[-1].rstrip().endswith(';') \
                and not cur[-1].lstrip().startswith(('//', '#')):
            out.append([start, cur, blank]); cur = []; blank = False
        if depth == 0 and not cur:
            start = i
        cur.append(l)
        depth += strip_code(l).count('{') - strip_code(l).count('}')
    if cur:
        out.append([start, cur, blank])
    res = []
    for (a, c, b) in out:
        kind = name = None
        for l in c:
            if l.startswith(('//', '#[', '#![')):
                continue
            m = ITEM.match(l)
            if m:
                kind, name = m.group(1), m.group(2)
                if kind == 'impl':
                    m2 = re.search(r'impl(?:<[^>]*>)?\s+(?:[\w:]+(?:<[^>]*>)?\s+for\s+)?([A-Za-z_]\w*)', l)
                    name = 'impl ' + (m2.group(1) if m2 else '?')
            break
        res.append({'line': a + 1, 'lines': c, 'blank': b, 'kind': kind, 'name': name})
    return res


# module -> (doc, [item names]); ORDER is the commit order.
MODS = {
    'io': ("Process primitives every part of the loader uses: writes to a descriptor, `exit_group`, reads\n"
           "//! of the initial stack and of C strings, and the fatal start-up diagnostics.",
           ['ARG_MAX_READ', 'write_all', 'exit', 'stack_word', 'c_string', 'refuse', 'lo32', 'fail_with',
            'fail_loading', 'no_memory']),
    'state': ("The loader's shared start-time state: the interpreter's own base, the table of start-time\n"
              "//! objects `link` records for `dlopen` (DL-16a; inline, then spilled to a mapping), and the bounds\n"
              "//! the parts share.",
              ['OWN_BASE', 'START_OBJS', 'START_SPILL', 'start_obj', 'START_INTERP', 'START_PAGE', 'START_ARGS',
               'AUXV_WORDS_MAX', 'PHDR_WORDS']),
    'tls': ("Thread-local storage (design §7): the provisional thread block, the static TLS block and its\n"
            "//! surplus for `dlopen`ed modules, the module table every new thread copies, the dtv and its\n"
            "//! generation, and `__tls_get_addr`.",
            ['TB_WORDS', 'Provisional', 'PROVISIONAL', 'GENERATION', 'STARTED_HERE', 'INITIAL_TAIL', '_',
             'map_area', 'program_tls', 'install_tls', 'THREAD_MODS', 'thread_mod', 'THREAD_DTV_WORDS',
             'THREAD_TLS_SIZE', 'THREAD_TLS_ALIGN', 'SURPLUS_LO', 'record_thread_tls', 'module_words',
             'dtv_slot', 'dtv_words', 'init_block', 'fill_static', 'NewTls', 'impl NewTls', 'surplus',
             'module_room', 'tls_generation', 'release_tls', 'initial_place', 'publish_tls', 'tls_visit',
             'thread_tls_init', 'thread_tls_release', '__tls_get_addr', 'tls_slow',
             '_dl_get_tls_static_info']),
    'image': ("Reading and mapping objects: opening files and identifying them, the views of mapped images\n"
              "//! the relocator reads, the program's own image, the system-call `Mapper` and relocation\n"
              "//! `Image`, the cache file, and the candidates of a search.",
              ['FileId', 'file_mode', 'open_file', 'scratch', 'read_image', 'live_segment', 'span_slots',
               'image_view', 'parse', 'own_object', 'ProcImage', 'impl ProcImage', 'AT_FDCWD', 'load',
               'SysMapper', 'impl SysMapper', 'word', 'MAP_PRIVATE_I32', 'open_whole', 'read_cache',
               'Unusable', 'open_candidate', 'dynamic_of', 'dyn_string', 'dir_of', 'VIEW_ALIGN', 'map_view']),
    'initfini': ("Constructors and exit finalisers of the start-time objects (DL-11, D-27.19): `run_init`\n"
                 "//! runs the initialisation functions after relocation and records the termination hooks\n"
                 "//! that `fini_all` runs at exit.",
                 ['fn_addr0', 'fn_addr', 'array_words', 'FINI_HOOKS', 'FINI_IDX', 'FINI_SPILL', 'run_init', 'fini_all']),
    'link': ("Program start's link (DL-6 to DL-10): the `DT_NEEDED` search from the program and the\n"
             "//! preloads (`LD_PRELOAD`, `/etc/ld.so.preload`), mapping, versions, relocation and symbol\n"
             "//! binding (`STB_GNU_UNIQUE` included), the TLS layout, the state `dlopen` reads later, and the\n"
             "//! debugger rendezvous's start-time link maps (DL-23; [`super::debug`] fills the structure).",
             ['link', 'DT_DEBUG', 'rendezvous_start', 'own_dynamic', 'interp_name', 'EDGES_INLINE', 'Origin',
              'impl Origin', 'Preloads', 'impl Preloads']),
    'trace': ("The dependency list (`--list`, `ldd`, `LD_TRACE_LOADED_OBJECTS`; DL-8, D-27.16): the search\n"
              "//! of program start, printed instead of run.",
              ['Obj', 'hex16', 'list']),
    'entry': ("Program start (DL-5, DL-6): `niji_ld_start`, the auxiliary vector and the environment, the\n"
              "//! secure-mode clear, rseq registration, the libc_nonshared report, and `run`, the start-up\n"
              "//! sequence up to entering the program.",
              ['__rseq_offset', '__rseq_size', '__rseq_flags', 'RSEQ_WANTED', 'REPORT_NONSHARED',
               'report_nonshared', 'register_rseq', 'read_auxv', 'store_words', 'clear_unsecure', 'env_entries',
               'niji_ld_start', 'program_headers', 'run', '__libc_stack_end', 'List', 'impl List']),
    'exec_stack': ("COMPAT-19, executable stacks (opt-in tunable `niji.compat.execstack`): whether the start\n"
                   "//! accepts an object that asks for one, the refusals, and making the main and thread\n"
                   "//! stacks executable before any constructor runs.",
                   ['EXEC_STACK', 'PROT_GROWSDOWN', 'exec_stack_accepted', 'refuse_exec_stack_program',
                    'exec_stack_dependency', 'exec_main_stack', 'exec_stacks_at_start', 'exec_stack_candidate']),
}
ORDER = ['io', 'state', 'tls', 'image', 'initfini', 'link', 'trace', 'entry', 'exec_stack']


# External `use` groups of the original file: (attr line or None, path prefix, [(name, ident)]).
EXT = None
# Extra identifiers that imply an import (traits used through methods or associated items).
IMPLIED = {'Arch': ['Native'], 'CWords': ['ctext'], 'GlobalAlloc': ['GlobalAlloc']}
# Visibility fix-ups the moves need: (module, old, new) applied to that module's text.
FIXUPS = [
    ('tls', '\n    mods: Table<[u64; 5], INLINE>,\n    generation: u64,', '\n    pub(super) mods: Table<[u64; 5], INLINE>,\n    generation: u64,'),
    ('tls', '\nstruct Provisional([AtomicUsize; TB_WORDS]);', '\npub(super) struct Provisional(pub(super) [AtomicUsize; TB_WORDS]);'),
    ('image', 'struct SysMapper {\n    fd: i32,', 'struct SysMapper {\n    pub(super) fd: i32,'),
    ('link', "    path: usize,\n    aux: Auxv<'a>,", "    pub(super) path: usize,\n    pub(super) aux: Auxv<'a>,"),
    ('link', "    env: &'static [u8],\n    file: &'static [u8],\n    file_path: &'a [u8],", "    pub(super) env: &'static [u8],\n    pub(super) file: &'static [u8],\n    pub(super) file_path: &'a [u8],"),
    ('entry', "    ld: &'a start::LdEnv<'static>,\n    secure: bool,", "    pub(super) ld: &'a start::LdEnv<'static>,\n    pub(super) secure: bool,"),
    ('entry', '    page: u64,\n    vdso: Option<u64>,\n    base: usize,', '    pub(super) page: u64,\n    pub(super) vdso: Option<u64>,\n    pub(super) base: usize,'),
    ('entry', "    fn ctx<'c>(&'c self", "    pub(super) fn ctx<'c>(&'c self"),
    ('link', '    fn names(&self)', '    pub(super) fn names(&self)'),
]
# Imports the ident scan finds but the module does not need (a field or local with an item's name).
DROP = {'': ['word'], 'image': ['Arch', 'table'], 'entry': ['elf', 'word'], 'link': ['refuse', 'Obj', 'word', 'table'], 'trace': ['refuse', 'interp_name'],
        'tls': ['word', 'init'], 'exec_stack': ['Arch']}
# The same for the root at one stage (stage -> names): at stage 6 the root still holds `trace`'s items
# (a local `interp_name`), at stage 7 `entry`'s (a parameter `elf`).
DROP_ROOT_AT = {6: ['interp_name'], 7: ['elf']}


CHILDREN = ['debug', 'dlopen', 'readers', 'table']


def child_needs():
    """Names the existing children of `ld` reach in `ld` itself: `super::` from ld/<child>.rs,
    `super::super::` from ld/<child>/*.rs (read from the tree, so a new base needs no list)."""
    names = set()
    for c in CHILDREN:
        files = [(os.path.join(SRC, 'ld', c + '.rs'), r'(?<!::)super::')]
        sub = os.path.join(SRC, 'ld', c)
        if os.path.isdir(sub):
            files += [(os.path.join(sub, f), r'(?<!::)super::super::') for f in sorted(os.listdir(sub))
                      if f.endswith('.rs')]
        for path, pre in files:
            text = '\n'.join(re.sub(r'//.*', '', l) for l in open(path).read().split('\n'))  # code, not doc links
            for group in re.findall(pre + r'\{([^}]*)\}', text):
                names |= {x.strip() for x in group.split(',') if x.strip()}
            names |= set(re.findall(pre + r'([A-Za-z_]\w*)', text))
    return names - {'super'}


def defined_names(ch):
    """Names one chunk defines (a chunk may hold several consts/statics)."""
    names = set()
    for l in ch['lines']:
        m = ITEM.match(l)
        if m and m.group(1) in ('fn', 'struct', 'enum', 'static', 'const', 'type') and m.group(2):
            names.add(m.group(2))
    return names


def idents(text):
    text = '\n'.join(re.sub(r'//.*', '', l) for l in text.split('\n'))
    text = re.sub(r'"(\\.|[^"\\])*"', '""', text)
    # an identifier used as a path, value or call; not a field/method (`.x`), not a path tail (`::x`)
    return set(re.findall(r'(?<![.\w])(?<!::)([A-Za-z_][A-Za-z0-9_]*)(?!\s*:[^:])', text))


def parse_uses(lines):
    groups, attr = [], None
    for l in lines:
        if l.startswith('#[cfg'):
            attr = l; continue
        m = re.match(r'^use (.*);$', l)
        if not m:
            attr = None; continue
        body = m.group(1)
        mb = re.match(r'^(.*)::\{(.*)\}$', body)
        if mb:
            prefix, items = mb.group(1), [x.strip() for x in mb.group(2).split(',')]
        else:
            prefix, items = body.rsplit('::', 1)[0], [body.rsplit('::', 1)[1]]
        pairs = []
        for it in items:
            if it == 'self':
                pairs.append(('self', prefix.rsplit('::', 1)[-1]))
            elif ' as ' in it:
                pairs.append((it, it.split(' as ')[1].strip()))
            else:
                pairs.append((it, it))
        groups.append((attr, prefix, pairs)); attr = None
    return groups


def fmt_use(attr, prefix, items):
    if len(items) == 1 and (items[0] == 'self' or items[0].startswith('self as ')):
        return ([attr] if attr else []) + [f'use {prefix}{items[0][4:]};']
    s = f'use {prefix}::{{{", ".join(items)}}};' if len(items) > 1 else f'use {prefix}::{items[0]};'
    return ([attr] if attr else []) + [s]


def ext_uses(text, groups, mod):
    ids = idents(text)
    out = []
    for attr, prefix, pairs in groups:
        keep = []
        for it, ident in pairs:
            need = ident in ids
            for trig in IMPLIED.get(ident, []):
                need = need or trig in ids or trig in text
            if ident in DROP.get(mod, ()):
                need = False
            if need:
                keep.append(it)
        if keep:
            # a root `use` through a sibling child (`table::...`) reads `super::table::...` below `ld`
            if mod and prefix.split('::')[0] in CHILDREN:
                prefix = 'super::' + prefix
            out += fmt_use(attr, prefix, keep)
    return out


def main():
    DROP[''] = DROP[''] + DROP_ROOT_AT.get(stage, [])
    lines = open(ORIG).read().split('\n')
    if lines and lines[-1] == '':
        lines = lines[:-1]
    cs = chunks(lines)
    owner = {}
    for m in ORDER[:stage]:
        for n in MODS[m][1]:
            owner[n] = m
    where = {}  # defined name -> module ('' = root)
    for ch in cs:
        nm = ch['name']
        m = owner.get(nm) if nm else None
        if nm and nm.startswith('impl '):
            m = owner.get(nm)
        ch['mod'] = m or ''
        for d in defined_names(ch) - {'_'}:
            where[d] = ch['mod']
    for c in CHILDREN:
        where[c] = ''
    unknown = [n for m in ORDER[:stage] for n in MODS[m][1] if n not in {c['name'] for c in cs}]
    if unknown:
        sys.exit(f'unknown items: {unknown}')
    use_lines = [l for ch in cs if ch['kind'] == 'use' for l in ch['lines']]
    groups = parse_uses(use_lines)
    texts = {m: '\n'.join(l for ch in cs if ch['mod'] == m and ch['kind'] != 'use' for l in ch['lines'])
             for m in [''] + ORDER[:stage]}
    # who needs what across modules
    need_vis = set()
    sib = {}
    for m in ORDER[:stage]:
        ids = idents(texts[m]) - set(DROP.get(m, ()))
        own = {d for d, w in where.items() if w == m}
        refs = sorted((ids & set(where)) - own)
        sib[m] = refs
        for r in refs:
            need_vis.add(r)
    root_ids = (idents(texts['']) - set(DROP[''])) | child_needs()
    root_refs = sorted(r for r in (root_ids | set(PUBLIC)) & set(where) if where[r] != '')
    need_vis |= set(root_refs)
    # emit modules
    os.makedirs(os.path.join(SRC, 'ld'), exist_ok=True)
    for m in ORDER[:stage]:
        body = []
        for ch in cs:
            if ch['mod'] != m or ch['kind'] == 'use':
                continue
            ls = list(ch['lines'])
            for i, l in enumerate(ls):
                mm = ITEM.match(l)
                if mm and mm.group(1) in ('fn', 'struct', 'enum', 'static', 'const', 'type') and mm.group(2) in need_vis \
                        and not l.startswith('pub'):
                    ls[i] = 'pub(super) ' + l
            if body and ch['blank']:
                body.append('')
            body += ls
        text = '\n'.join(body)
        for (fm, old, new) in FIXUPS:
            if fm == m:
                if old not in text:
                    sys.exit(f'fixup not found in {m}: {old}')
                text = text.replace(old, new)
        head = ['//! ' + MODS[m][0], ''] + ext_uses(text, groups, m)
        bymod = {}
        for r in sib[m]:
            bymod.setdefault(where[r], []).append(r)
        for w in sorted(bymod):
            items = sorted(bymod[w])
            head += fmt_use(None, 'super' if w == '' else f'super::{w}', items)
        open(os.path.join(SRC, 'ld', m + '.rs'), 'w').write('\n'.join(head) + '\n\n' + text + '\n')
    # emit root
    out, emitted_imports = [], False
    for ch in cs:
        if ch['mod'] != '':
            continue
        ls = list(ch['lines'])
        if ch['kind'] == 'use':
            if emitted_imports:
                continue
            emitted_imports = True
            ls = ext_uses(texts[''], groups, '')
            # root re-imports: moved items the root's own code or the children use
            bymod = {}
            for r in root_refs:
                bymod.setdefault(where[r], []).append(r)
            pubs = []
            for w in ORDER[:stage]:
                if w in bymod:
                    names = sorted(bymod[w])
                    for vis in ('', 'pub(crate) ', 'pub '):
                        part = [n for n in names if PUBLIC.get(n, '') == vis]
                        if part:
                            (pubs if vis else ls).extend(
                                vis + u for u in fmt_use(None, 'self::' + w, part))
            if pubs:
                ls += [''] + pubs
        if out and ch['blank']:
            out.append('')
        if ch['kind'] == 'mod' and 'mod dlopen;' in ls:
            ls = [f'mod {m};' for m in ORDER[:stage]] + ls if not any(l.startswith('mod io;') for l in ls) else ls
        out += ls
    root = '\n'.join(out) + '\n'
    # `c_void` is left only to `ld_mem`, which a build with `export` leaves out (as `c_int`)
    old = '#[cfg(not(feature = "export"))]\nuse core::ffi::c_int;\nuse core::ffi::c_void;\n'
    if old in root and 'c_void' not in root.split('macro_rules! ld_mem')[0].replace(old, ''):
        root = root.replace(old, '#[cfg(not(feature = "export"))]\nuse core::ffi::{c_int, c_void};\n')
    open(os.path.join(SRC, 'ld.rs'), 'w').write(root)


PUBLIC = {}
if __name__ == '__main__':
    # items that are `pub` or `pub(crate)` in the original file: re-exported at their old path
    for l in open(ORIG):
        mm = ITEM.match(l)
        if mm and l.startswith(('pub ', 'pub(crate) ')) and mm.group(2):
            PUBLIC[mm.group(2)] = 'pub ' if l.startswith('pub ') else 'pub(crate) '
    main()
