#!/usr/bin/env python3
"""Probe the types and values the public headers give their names (plan/headers.md slice 3, Appendix C.13).

    python3 spec/tools/type_probe.py [--jobs N] [--headers a.h,b.h] [--out DIR]

Run on the reference system (Appendix C.0), after header_probe.py: the names and the header that shows each
one come from `spec/abi/visibility.tsv`. Writes two tables into spec/abi/ (or --out):

* `typedefs.tsv`  kind, name, header, size, align, cxx: per typedef, struct/union/enum tag, data object and
  function, the size and alignment (C) and the C++ spelling of its type with every typedef resolved (the
  spelling that enters C++ linkage names: `FILE` is spelled through its tag). For an object, `cxx` is its
  declared type (`decltype(O)`); for a function, its function type with the parameter typedefs resolved.
* `macros.tsv`    name, header, type, value: the C type of each integer constant (`_Generic`) with its
  value, and the type and value of each floating macro (hexadecimal, exact) and string macro.

Clean room (plan/headers.md section 2.2, ruling R-H0). A probe unit includes one public header and declares
objects and functions of its own, one test per line. The probe reads back exactly three things: the line
numbers of errors in its own file (a failing test line is dropped, its name is then not of that form), the
bytes of its own objects in the C object file (parsed here from the ELF symbol table), and the demangled
names of its own C++ functions (`nm -C`). It never reads diagnostic text, `-E` or `-dM` output, or any
header text.

Configurations: C uses `gnu` (`-std=gnu17 -D_GNU_SOURCE`), C++ uses `cxx14` (C++14: `noexcept` is not part
of a function type there, so it stays out of `cxx`; slice 4's noexcept.tsv records it). A name is probed in
the empty unit when the compiler alone shows it, else in the first header (in sorted order) that shows it
in that configuration with that kind.
"""
import argparse, csv, os, re, shutil, struct, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import header_probe as hp  # noqa: E402  (Unit, error_lines, CONFIGS, BASE)

ABI = hp.ABI
CFG = {c[0]: c for c in hp.CONFIGS}
C_CFG, CXX_CFG = CFG["gnu"], CFG["cxx14"]
# Errors inside a macro expansion are reported at the expansion point, that is on the probe's own line.
EXTRA = ["-ftrack-macro-expansion=0", "-fno-common"]
TYPE_KINDS = ("typedef", "struct", "union", "enum", "object", "function")
PREFIX = {"typedef": "tp_ty_", "struct": "tp_st_", "union": "tp_un_", "enum": "tp_en_", "object": "tp_ob_", "function": "tp_fn_"}
INT_TYPES = ["_Bool", "char", "signed char", "unsigned char", "short", "unsigned short", "int", "unsigned int", "long",
             "unsigned long", "long long", "unsigned long long", "__int128", "unsigned __int128"]
# name -> (bytes in the object, exponent bits, stored fraction bits, explicit integer bit)
FLOAT_TYPES = {"float": (4, 8, 23, False), "double": (8, 11, 52, False), "long double": (10, 15, 63, True),
               "_Float32": (4, 8, 23, False), "_Float64": (8, 11, 52, False), "_Float32x": (8, 11, 52, False),
               "_Float64x": (10, 15, 63, True), "_Float128": (16, 15, 112, False)}
FLOAT_ORDER = list(FLOAT_TYPES)
INT_SEL = ", ".join(f"{t}: {i + 1}" for i, t in enumerate(INT_TYPES))
FLOAT_SEL = ", ".join(f"{t}: {i + 1}" for i, t in enumerate(FLOAT_ORDER))
# Not header facts: compiler macros whose value depends on the probe unit itself (its path, line, build time),
# the type keywords visibility.tsv's empty unit shows as typedefs (`long *p;` declares), and `main`.
POSITIONAL = {"__FILE__", "__LINE__", "__DATE__", "__TIME__", "__TIMESTAMP__", "__COUNTER__", "__BASE_FILE__",
              "__FILE_NAME__", "__INCLUDE_LEVEL__", "long", "short", "signed", "unsigned", "main"}
MISSING = object()


def in_config(configs, cid, ids):
    if configs == "*":
        return True
    if configs.startswith("!"):
        return cid not in configs[1:].split(",")
    return cid in configs.split(",")


def assignments(ids):
    """{lang: {header or None: {kind: [names]}}} from visibility.tsv: where each name is probed."""
    rows = hp.tsv("visibility.tsv")
    plan = {}
    for lang, cfg, kinds in (("c", C_CFG, ("typedef", "struct", "union", "enum", "object", "macro", "enumerator")),
                             ("c++", CXX_CFG, ("typedef", "struct", "union", "enum", "object", "function"))):
        where = {}
        for r in rows:
            if r["kind"] not in kinds or not in_config(r["configs"], cfg[0], ids):
                continue
            # Integer, floating and string constants are one class whatever their declaration kind.
            kind = "macro" if r["kind"] == "enumerator" else r["kind"]
            if r["name"] in POSITIONAL:
                continue
            h = None if r["header"] == "-" else r["header"]
            key = (kind, r["name"])
            cur = where.get(key, MISSING)
            if cur is MISSING or h is None or (cur is not None and h < cur):
                where[key] = h
        per = {}
        for (kind, name), h in where.items():
            per.setdefault(h, {}).setdefault(kind, []).append(name)
        plan[lang] = per
    return plan


def compile_clean(unit, lang, flags, path, obj):
    """Compile `unit` to `obj` after dropping its failing test lines; the keys of the tests that compiled."""
    flags = flags + " " + " ".join(EXTRA)
    _, failed = unit.run(lang, flags, path)
    keep = {}
    for line, key in unit.tests.items():
        if key in failed:
            unit.lines[line - 1] = ""
        else:
            keep[line] = key
    with open(path, "w") as f:
        f.write("\n".join(unit.lines) + "\n")
    cmd = [hp.COMPILER[lang]] + flags.split() + ["-w", "-O0", "-c", "-x", "c" if lang == "c" else "c++", path, "-o", obj]
    p = subprocess.run(cmd, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if p.returncode != 0:
        raise RuntimeError(f"{os.path.basename(path)}: the unit does not compile after its failing tests were dropped")
    return set(keep.values())


def elf_objects(path, prefix="tp_"):
    """{symbol: bytes} for the probe's own defined data symbols, read from the ELF64 object's symbol table."""
    b = open(path, "rb").read()
    if b[:4] != b"\x7fELF" or b[4] != 2 or b[5] != 1:
        raise RuntimeError(f"{path}: not a little-endian ELF64 object")
    shoff, = struct.unpack_from("<Q", b, 0x28)
    shentsize, shnum = struct.unpack_from("<HH", b, 0x3a)
    secs = [struct.unpack_from("<IIQQQQIIQQ", b, shoff + i * shentsize) for i in range(shnum)]
    relocated = {s[7] for s in secs if s[1] in (4, 9)}  # SHT_RELA / SHT_REL: sh_info is the section relocated
    out = {}
    for s in secs:
        if s[1] != 2:  # SHT_SYMTAB
            continue
        stroff = secs[s[6]][4]
        for j in range(s[5] // 24):
            st_name, _info, _other, shndx, value, size = struct.unpack_from("<IBBHQQ", b, s[4] + j * 24)
            end = b.index(b"\0", stroff + st_name)
            name = b[stroff + st_name:end].decode()
            if not name.startswith(prefix) or shndx == 0 or shndx >= 0xff00:
                continue
            if shndx in relocated:
                raise RuntimeError(f"{path}: {name} lies in a relocated section; its bytes are not final")
            sec = secs[shndx]
            out[name] = bytes(size) if sec[1] == 8 else b[sec[4] + value:sec[4] + value + size]
    return out


def demangled(path):
    """{function name: template argument} for the probe's own C++ functions `tp_xx_N(tp_w<ARG>*)`."""
    text = subprocess.run(["nm", "-C", "--defined-only", path], capture_output=True, text=True, check=True).stdout
    out = {}
    for line in text.splitlines():
        m = re.match(r"^[0-9a-f]+ T (tp_\w+)\(tp_w<(.*)>\*\)$", line)
        if m:
            out[m.group(1)] = m.group(2)
    return out


def hexfloat(sign, m, e):
    """The exact value sign * m * 2**e as a normalised C hexadecimal floating constant."""
    s = "-" if sign else ""
    if m == 0:
        return s + "0x0p+0"
    n = m.bit_length() - 1
    frac = m - (1 << n)
    pad = (-n) % 4
    digits = format(frac << pad, f"0{(n + pad) // 4}x").rstrip("0") if n else ""
    return f"{s}0x1{'.' + digits if digits else ''}p{e + n:+d}"


def float_value(tname, data):
    nbytes, ebits, fbits, explicit = FLOAT_TYPES[tname]
    v = int.from_bytes(data[:nbytes], "little")
    total = ebits + fbits + (1 if explicit else 0)
    sign = v >> total & 1
    exp = v >> (fbits + (1 if explicit else 0)) & ((1 << ebits) - 1)
    frac = v & ((1 << fbits) - 1)
    bias = (1 << (ebits - 1)) - 1
    if exp == (1 << ebits) - 1:
        return ("-" if sign else "") + ("inf" if frac == 0 else "nan")
    if exp == 0:
        return hexfloat(sign, frac, 1 - bias - fbits)
    return hexfloat(sign, frac | (1 << fbits), exp - bias - fbits)


def c_string(data):
    out = []
    for ch in data:
        if ch in (0x22, 0x5c):
            out.append("\\" + chr(ch))
        elif 0x20 <= ch < 0x7f:
            out.append(chr(ch))
        else:
            out.append(f"\\{ch:03o}")
    return '"' + "".join(out) + '"'


def probe_c(header, kinds, names_undef, work):
    """C unit: sizes and alignments of types and objects, and the integer, floating and string constants."""
    tag = (header or "-").replace("/", "_")
    u = hp.Unit(header)
    for n in sorted(kinds.get("macro", [])):
        u.test(("int", n), f"_Static_assert(_Generic(({n}), {INT_SEL}, default: 0), \"i\"); "
                           f"const unsigned long long tp_i_{n}[2] = {{ _Generic(({n}), {INT_SEL}), (unsigned long long)({n}) }};")
        u.test(("flt", n), f"_Static_assert(_Generic(({n}), {FLOAT_SEL}, default: 0), \"f\"); "
                           f"const int tp_k_{n} = _Generic(({n}), {FLOAT_SEL}); const __typeof__({n}) tp_f_{n} = ({n});")
        u.test(("str", n), f"_Static_assert(_Generic(({n}), char *: 1, default: 0), \"s\"); const char tp_s_{n}[] = {n};")
    for n in sorted(names_undef):
        u.raw(f"#undef {n}")
    for kind in ("typedef", "struct", "union", "enum"):
        for n in sorted(kinds.get(kind, [])):
            t = n if kind == "typedef" else f"{kind} {n}"
            # An array of T is an error for an incomplete type, void and a function type, whose sizeof GNU C
            # would otherwise give as 1.
            u.test((kind, n), f"extern {t} tp_c{PREFIX[kind][3:5]}_{n}[1]; "
                              f"const unsigned long long {PREFIX[kind]}{n}[2] = {{ sizeof({t}), _Alignof({t}) }};")
    for n in sorted(kinds.get("object", [])):
        u.test(("object", n), f"const unsigned long long tp_ob_{n}[2] = {{ sizeof({n}), __alignof__({n}) }};")
    path = os.path.join(work, f"{tag}.c")
    obj = path + ".o"
    compile_clean(u, "c", C_CFG[2], path, obj)
    data = elf_objects(obj)
    sizes, macros = {}, {}
    for kind in ("typedef", "struct", "union", "enum", "object"):
        for n in kinds.get(kind, []):
            d = data.get(PREFIX[kind] + n)
            if d is not None:
                sizes[(kind, n)] = struct.unpack("<QQ", d)
    for n in kinds.get("macro", []):
        if f"tp_i_{n}" in data:
            code, raw = struct.unpack("<QQ", data[f"tp_i_{n}"])
            t = INT_TYPES[code - 1]
            unsigned = t.startswith("unsigned") or t == "_Bool"
            # The value was converted to 64 bits; a signed type reads it back as two's complement.
            val = raw if unsigned or raw < 1 << 63 else raw - (1 << 64)
            macros[n] = (t, str(val))
        elif f"tp_k_{n}" in data:
            t = FLOAT_ORDER[struct.unpack("<i", data[f"tp_k_{n}"])[0] - 1]
            macros[n] = (t, float_value(t, data[f"tp_f_{n}"]))
        elif f"tp_s_{n}" in data:
            s = data[f"tp_s_{n}"]
            macros[n] = (f"char[{len(s)}]", c_string(s[:-1]))
    return sizes, macros


def probe_cxx(header, kinds, names_undef, work):
    """C++ unit: the resolved spelling of each type, object type and function type, through `nm -C`."""
    tag = (header or "-").replace("/", "_")
    u = hp.Unit(header)
    u.raw("template<class T> struct tp_w {};")
    for n in sorted(names_undef):
        u.raw(f"#undef {n}")
    for kind in TYPE_KINDS:
        for n in sorted(kinds.get(kind, [])):
            t = {"typedef": n, "object": f"decltype({n})", "function": f"decltype({n})"}.get(kind, f"{kind} {n}")
            u.test((kind, n), f"void {PREFIX[kind]}{n}(tp_w<{t}>*) {{}}")
    path = os.path.join(work, f"{tag}.cc")
    obj = path + ".o"
    compile_clean(u, "c++", CXX_CFG[2], path, obj)
    spelled = demangled(obj)
    out = {}
    for kind in TYPE_KINDS:
        for n in kinds.get(kind, []):
            if PREFIX[kind] + n in spelled:
                out[(kind, n)] = spelled[PREFIX[kind] + n]
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    ap.add_argument("--headers", default="")
    ap.add_argument("--out", default=ABI)
    args = ap.parse_args()
    ids = [c[0] for c in hp.CONFIGS]
    plan = assignments(ids)
    only = set(args.headers.split(",")) if args.headers else None
    jobs = [(lang, h) for lang in ("c", "c++") for h in sorted(plan[lang], key=lambda x: x or "")
            if only is None or (h or "-") in only]
    work = tempfile.mkdtemp(prefix="type_probe.")

    def run(job):
        lang, h = job
        kinds = plan[lang][h]
        undef = {n for k, ns in kinds.items() if k != "macro" for n in ns}
        sub = os.path.join(work, lang)
        return (probe_c if lang == "c" else probe_cxx)(h, kinds, undef, sub)

    try:
        for lang in ("c", "c++"):
            os.makedirs(os.path.join(work, lang))
        with ThreadPoolExecutor(max_workers=args.jobs) as ex:
            results = list(ex.map(run, jobs))
    finally:
        shutil.rmtree(work, ignore_errors=True)
    sizes, macros, spelled, header_of = {}, {}, {}, {}
    for (lang, h), res in zip(jobs, results):
        for kind, names in plan[lang][h].items():
            for n in names:
                header_of.setdefault((lang, kind, n), h or "-")
        if lang == "c":
            sizes.update(res[0])
            macros.update(res[1])
        else:
            spelled.update(res)
    trows = []
    for kind, n in sorted(set(sizes) | set(spelled), key=lambda k: (k[1], TYPE_KINDS.index(k[0]))):
        size, align = sizes.get((kind, n), ("", ""))
        header = header_of.get(("c", kind, n)) or header_of[("c++", kind, n)]
        trows.append((kind, n, header, str(size), str(align), spelled.get((kind, n), "")))
    mrows = [(n, header_of[("c", "macro", n)], t, v) for n, (t, v) in sorted(macros.items())]
    os.makedirs(args.out, exist_ok=True)
    with open(os.path.join(args.out, "typedefs.tsv"), "w", newline="\n") as f:
        f.write("kind\tname\theader\tsize\talign\tcxx\n")
        for r in trows:
            f.write("\t".join(r) + "\n")
    with open(os.path.join(args.out, "macros.tsv"), "w", newline="\n") as f:
        f.write("name\theader\ttype\tvalue\n")
        for r in mrows:
            f.write("\t".join(r) + "\n")
    asked = {lang: sum(len(v) for h, ks in plan[lang].items() if only is None or (h or "-") in only
                       for k, v in ks.items()) for lang in plan}
    print(f"units {len(jobs)}  C names {asked['c']}  C++ names {asked['c++']}  typedefs.tsv rows {len(trows)}"
          f"  macros.tsv rows {len(mrows)} (integer {sum(1 for r in mrows if r[2] in INT_TYPES)},"
          f" floating {sum(1 for r in mrows if r[2] in FLOAT_TYPES)}, string {sum(1 for r in mrows if r[2].startswith('char['))})")


if __name__ == "__main__":
    sys.exit(main())
