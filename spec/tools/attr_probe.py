#!/usr/bin/env python3
"""Probe which diagnostic attributes each function carries (plan/headers.md slice 19, Appendix C.14).

    python3 spec/tools/attr_probe.py table [--include DIR] [--out FILE] [--keep DIR]
    python3 spec/tools/attr_probe.py check --include DIR [--keep DIR]

`table` compiles a corpus against the system's public headers (the reference) and writes
`spec/abi/diagattrs.tsv`: for each function that libniji's headers (DIR, default libc-include/) declare,
the attributes the reference's declaration has. `check` compiles the same corpus twice, against the
reference and against libniji's headers (`-nostdinc -isystem DIR -isystem <the compiler's include>`), and
prints each probe line on which the two differ (rc 1 when there is one). Run it on the reference system.

Clean room (plan/headers.md section 2.2, ruling R-H0). The probe compiles translation units of its own in
which every test is one line, and reads back only:
* for each diagnostic located in the probe's own file, its LINE NUMBER and its option name (`-Wnonnull`),
  through `-fdiagnostics-format=json`; the diagnostic text is dropped by the reader and never printed;
* the undefined symbols of the probe's own object file (`nm -u`): a test calls an extern marker function
  `mk_*` under a condition the optimizer can decide only if the declaration has the attribute, so the
  marker is referenced exactly when it does not.
It never reads `-E` or `-dM` output, disassembly or any header text.

Tests per function F (parameters from prototypes.tsv: a pointer is `v[k]`, an arithmetic value is a literal,
a `va_list` is an extern object; a function with a by-value aggregate parameter is not probed):
* `r`   F(valid arguments) with the result unused: -Wunused-result, -Wdeprecated-declarations, -Wunused-value
        (a pure function).
* `n.k` F with a null pointer constant for pointer parameter k: -Wnonnull.
* `f.j` F with an unknown `char *` for the `const char *` parameter j (and, for a variadic F, the same
        parameter replaced by the literal "%d" with a string argument in a second test `g.j`): -Wformat*.
* `h.j` a non-variadic F with the unknown conversion "%y" for the `const char *` parameter j: -Wformat= (the
        `v*printf` and `v*scanf` forms take a `va_list`, which the nonliteral test cannot see).
* `s.c` a variadic F with c null pointers appended: -Wformat= (a missing sentinel) tells the attribute and
        its position (c - 1 is the position of the first that makes it quiet).
* `nr`  F(valid arguments); mk_nr(): the marker is dropped when F is noreturn.
* `ct`  a non-variadic F with a result: two calls with a memory clobber between them, mk_ct() when the results
        differ: the marker is dropped when F is const (a pure F is called again; this tells pure from const).
* `e`   a non-variadic F with a result, in a unit without the `bp` redeclarations: errno = 0; r = F(opaque
        arguments); mk_e() when errno != 0. The marker is
        dropped when the compiler may assume F leaves errno alone (pure or const). Where the reference keeps it, its
        objects let the call set errno, and the table marks F neither pure nor const, whatever the other tests say
        (they use literal arguments, which the compiler's own knowledge of a builtin such as acos can fold).
* `ma`  two calls p = F(..), q = F(..), stores *p = 1 and *q = 2; the marker is called when *p != 1, so it is
        dropped when the two results cannot alias (attribute malloc).
* `as.S` p = F(..) with the arguments of subset S set to 2 and the others opaque; the marker is dropped
        when __builtin_object_size(p, 0) is known to be 2^|S| (attribute alloc_size over S).

diagattrs.tsv (`name nonnull wur noreturn format malloc alloc_size deprecated sentinel pure`): nonnull is the comma list of
1-based parameters, format is `printf:IDX:FIRST` (kind from the function's name: scanf, strftime, strfmon or
printf), alloc_size the comma list of parameters, the others `1` or empty; pure is `1`, or `const` for a function the reference declares with attribute const.
"""
import argparse, csv, json, os, re, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
ABI = os.path.join(ROOT, "spec", "abi")
# libgen.h: its include turns `basename` into the XPG function (a macro), so a corpus that includes it probes another function.
SKIP_HEADERS = {"features.h", "features-time64.h", "stdc-predef.h", "gnu-versions.h", "assert.h", "libgen.h"}
OPT = ["-O2", "-std=gnu17", "-Wall", "-Wextra", "-Wformat=2", "-Wformat-security"]
# open and kin take an optional mode, not a sentinel list: the reference's checking wrappers report an error, inside a
# header and so on no test line, for a call with extra arguments. They get no sentinel test.
NO_SENTINEL = {"open", "open64", "openat", "openat64", "creat", "creat64"}
# The tests that read back a marker function (mk_<kind>_<line>) from the object.
MARKED = ("nr", "ma", "as", "ct", "e")


def protos():
    rows = {}
    with open(os.path.join(ABI, "prototypes.tsv")) as f:
        for r in csv.DictReader(f, delimiter="\t"):
            rows[r["name"]] = r["declaration"]
    return rows


def split_params(decl, name):
    i = decl.index("(", decl.index(name) + len(name) - 1)
    depth, cur, out = 0, "", []
    for c in decl[i + 1:]:
        if c == "(":
            depth += 1
        elif c == ")":
            if depth == 0:
                break
            depth -= 1
        if c == "," and depth == 0:
            out.append(cur.strip())
            cur = ""
        else:
            cur += c
    out.append(cur.strip())
    ret = decl[:i].replace("extern ", "")
    return ret, out


def classify(p):
    if p == "...":
        return "var"
    if "va_list" in p or "__va_list_tag" in p:
        return "ap"
    if "(" in p:
        return "fn"
    if "*" in p or "[" in p:
        return "constchar" if re.match(r"^const\s+char\s*\*", p) else "ptr"
    if re.search(r"\b(struct|union)\b|cookie_io_functions_t", p):
        return "agg"
    if re.search(r"locale_t\b", p):
        return "loc"
    return "num"


def declared(inc_dir, names):
    """Function names declared by the headers in inc_dir, and the headers."""
    heads = sorted(h for h in os.listdir(inc_dir) if h.endswith(".h") and h not in SKIP_HEADERS)
    # A header the reference itself refuses to include (a deliberate #error, as for <regexp.h>) cannot be part of the
    # corpus: one such line would keep the whole translation unit from building. Its functions have no row.
    heads = [h for h in heads if subprocess.run(["gcc", "-fsyntax-only", "-x", "c", "-"], input="#define _GNU_SOURCE 1\n#include <%s>\n" % h,
                                                capture_output=True, text=True).returncode == 0]
    found = set()
    for h in heads:
        with open(os.path.join(inc_dir, h)) as f:
            lines = [l for l in f if l.startswith("extern ")]
        text = "\n".join(lines)
        for n in names:
            if n in found:
                continue
            if re.search(r"\b%s \(" % re.escape(n), text):
                found.add(n)
    return sorted(found), heads


def corpus(names, heads, pr):
    # unistd.h first: with fcntl.h included before it, the reference's lockf loses its warn_unused_result (the header
    # order changes the answer), and a program that includes unistd.h alone sees it.
    order = ["unistd.h"] + [h for h in heads if h != "unistd.h"] if "unistd.h" in heads else heads
    L = ["#define _GNU_SOURCE 1"] + ["#include <%s>" % h for h in order] + ["#include <errno.h>"]
    L += ["extern void *v[16]; extern char *nl; extern int ov; extern long double oe; extern __builtin_va_list ap;"]
    meta = {}

    def add(fmt, key):
        """One test per line; `%(N)d` in fmt is the line's own number (it names the line's marker)."""
        n = len(L) + 1
        mk = "extern void mk_%s_%d(void); " % (key[1], n) if key[1] in MARKED else ""
        L.append(mk + fmt.replace("@N@", str(n)))
        meta[n] = key

    for n in names:
        ret, ps = split_params(pr[n], n)
        if ps == ["void"]:
            ps = []
        kinds = [classify(p) for p in ps]
        if "agg" in kinds or "(*" in ret or "[" in ret:
            continue
        fixed = [k for k in kinds if k != "var"]
        variadic = "var" in kinds
        isptr_ret = "*" in ret
        # ret ends with the function's name: the return type is what precedes it.
        has_result = re.sub(r"\b%s\s*$" % re.escape(n), "", ret).strip() != "void"
        # The `e` test's opaque argument per parameter: a floating one gets a long double (an int would let the compiler
        # know lround's argument integral and fold the call), any other number an int.
        opaque = {j: ("oe" if re.search(r"\b(double|float|_Float\d+x?)\b", p) else "ov")
                  for j, (k, p) in enumerate((k, p) for k, p in zip(kinds, ps) if k != "var") if k == "num"}

        def args(over=None, lit=True):
            o = []
            for j, k in enumerate(fixed):
                if over and j in over:
                    o.append(over[j])
                elif k == "ptr":
                    o.append("v[%d]" % j)
                elif k == "loc":
                    o.append("((locale_t) v[%d])" % j)
                elif k == "constchar":
                    o.append('"x"' if lit else "v[%d]" % j)
                elif k == "fn":
                    o.append("0")
                elif k == "ap":
                    o.append("ap")
                else:
                    o.append("1")
            return ", ".join(o)

        add("void r_@N@(void) { %s(%s); }" % (n, args()), (n, "r"))
        for j, k in enumerate(fixed):
            if k in ("ptr", "constchar", "fn"):
                add("void n_@N@(void) { %s(%s); }" % (n, args({j: "((void *) 0)"})), (n, "n", j + 1))
        for j, k in enumerate(fixed):
            if k == "constchar":
                add("void f_@N@(void) { %s(%s); }" % (n, args({j: "nl"})), (n, "f", j + 1))
                if not variadic:
                    add('void h_@N@(void) { %s(%s); }' % (n, args({j: '"%y"'})), (n, "h", j + 1))
                if variadic and j == len(fixed) - 1:
                    add('void g_@N@(void) { %s(%s, "s"); }' % (n, args({j: '"%d"'})), (n, "g", j + 1))
        if variadic and n not in NO_SENTINEL:
            for c in (1, 2):
                add("void s%d_@N@(void) { %s(%s%s); }" % (c, n, args(), ", ((void *) 0)" * c), (n, "s", c))
        add("void nr_@N@(void) { %s(%s); mk_nr_@N@(); }" % (n, args(lit=False)), (n, "nr"))
        if not variadic and has_result:
            # Opaque arguments: no builtin folding, so only the declaration decides.
            add("void e_@N@(void) { static __typeof__ (%s(%s)) volatile r; errno = 0; r = %s(%s); if (errno != 0) mk_e_@N@(); }"
                % (n, args(lit=False), n, args(opaque, lit=False)),
                (n, "e"))
        # A redeclaration with attribute pure: the compiler objects when it knows the function as a const builtin
        # (the math functions), and then the declaration must say const to compile at all.
        add("extern __typeof__ (%s) %s __attribute__ ((__pure__));" % (n, n), (n, "bp"))
        if not variadic and ret.replace("extern", "").strip() != "void" and all(k == "num" for k in fixed):
            # Two calls with a memory clobber between them: the results are known equal only when the attribute is const.
            # Only for numeric parameters, with literal arguments (a pointer argument reads memory, so cannot be const).
            add("void ct_@N@(void) { __typeof__ (%s(%s)) a = %s(%s); __asm__ volatile (\"\" ::: \"memory\"); __typeof__ (a) b = %s(%s);"
                " if (__builtin_memcmp (&a, &b, sizeof a) != 0) mk_ct_@N@(); }"
                % (n, args(), n, args(), n, args()),
                (n, "ct"))
        if isptr_ret:
            add("void ma_@N@(void) { char *p = (char *) %s(%s); char *q = (char *) %s(%s); *p = 1; *q = 2; if (*p != 1) mk_ma_@N@(); }"
                % (n, args(lit=False), n, args(lit=False)), (n, "ma"))
            nums = [j for j, k in enumerate(fixed) if k == "num"]
            for m in range(1, 1 << len(nums)):
                S = [nums[b] for b in range(len(nums)) if m >> b & 1]
                over = {j: ("2" if j in S else "ov") for j in nums}
                add("void as_@N@(void) { char *p = (char *) %s(%s); if (__builtin_object_size (p, 0) != %d) mk_as_@N@(); }"
                    % (n, args(over, lit=False), 1 << len(S)), (n, "as", ",".join(str(j + 1) for j in S)))
    return "\n".join(L) + "\n", meta


HEADER_ERRORS = []


def compile_side(text, inc, keep, tag):
    d = keep or tempfile.mkdtemp(prefix="attr-")
    os.makedirs(d, exist_ok=True)
    src = os.path.join(d, tag + ".c")
    obj = os.path.join(d, tag + ".o")
    with open(src, "w") as f:
        f.write(text)
    cmd = ["gcc"] + OPT + ["-fdiagnostics-format=json", "-c", src, "-o", obj]
    if inc:
        gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
        # The kernel UAPI headers (asm/, asm-generic/, linux/) are not part of the C library: libniji's headers include
        # them (pty.h: asm/ioctls.h). Only those three directories are made visible, through a directory of symbolic
        # links, so that no header of the reference's C library can stand in for one libniji lacks.
        uapi = os.path.join(d, "uapi")
        os.makedirs(uapi, exist_ok=True)
        for name, src_dir in (("asm", "/usr/include/x86_64-linux-gnu/asm"), ("asm-generic", "/usr/include/asm-generic"),
                              ("linux", "/usr/include/linux")):
            link = os.path.join(uapi, name)
            if not os.path.lexists(link) and os.path.isdir(src_dir):
                os.symlink(src_dir, link)
        cmd[1:1] = ["-nostdinc", "-isystem", os.path.abspath(inc), "-isystem", gccinc, "-isystem", uapi]
    p = subprocess.run(cmd, stdin=subprocess.DEVNULL, capture_output=True, text=True)
    diags = {}
    errs = set()
    try:
        data = json.loads(p.stderr[p.stderr.index("["):]) if "[" in p.stderr else []
    except ValueError:
        data = []
    for dg in data:
        locs = dg.get("locations") or []
        if not locs:
            continue
        c = locs[0].get("caret", {})
        if os.path.basename(c.get("file", "")) != tag + ".c":
            # An error inside an included header fails the whole corpus. Only libniji's own headers (inc given) are
            # quoted: the reference side keeps nothing but lines and option names (plan/headers.md section 2.2).
            if inc and dg.get("kind") in ("error", "fatal error") and len(HEADER_ERRORS) < 5:
                HEADER_ERRORS.append("%s:%s: %s" % (os.path.relpath(c.get("file", "?"), os.path.abspath(inc)), c.get("line"), dg.get("message", "")))
            continue
        opt = dg.get("option") or ""
        if dg.get("kind") == "error":
            errs.add(c.get("line"))
        diags.setdefault(c.get("line"), set()).add(opt or dg.get("kind", ""))
    if not os.path.exists(obj) or errs:
        if os.path.exists(obj):
            os.remove(obj)
        return diags, None, errs
    nm = subprocess.run(["nm", "-u", obj], capture_output=True, text=True, check=True).stdout.split()
    marks = set()
    for s in nm:
        m = re.match(r"^mk_(%s)_(\d+)$" % "|".join(MARKED), s)
        if m:
            marks.add(int(m.group(2)))
    return diags, marks, errs


def results(text, meta, inc, keep, tag):
    """The `e` tests in a unit of their own (the same line numbers, every other test line blank): GCC merges the `bp`
    redeclaration's pure into the one declaration of F for the whole unit, wherever the line is."""
    lines = text.split("\n")
    ekeys = {ln for ln, key in meta.items() if key[1] == "e"}
    main = "\n".join("" if ln + 1 in ekeys else x for ln, x in enumerate(lines))
    errno = "\n".join("" if ln + 1 in meta and ln + 1 not in ekeys else x for ln, x in enumerate(lines))
    res = results_unit(main, {ln: k for ln, k in meta.items() if ln not in ekeys}, inc, keep, tag)
    res.update(results_unit(errno, {ln: meta[ln] for ln in ekeys}, inc, keep, tag + "-e"))
    return res


def results_unit(text, meta, inc, keep, tag):
    """Compile; a test line that does not compile is blanked (keeping every line number) and recorded."""
    dropped = set()
    for _ in range(6):
        lines = text.split("\n")
        for ln in dropped:
            lines[ln - 1] = ""
        diags, marks, errs = compile_side("\n".join(lines), inc, keep, tag)
        if errs is None or not (errs - dropped):
            break
        dropped |= {e for e in errs if e in meta}
    if marks is None:
        sys.exit("attr_probe: the %s corpus does not build outside its test lines (lines %s)%s"
                 % (tag, sorted(errs - dropped)[:10], "".join("\n  compiler error: " + e for e in HEADER_ERRORS)))
    errs = dropped
    res = {}
    for ln, key in meta.items():
        flags = set(diags.get(ln, set()))
        if key[1] in MARKED:
            flags.add("marker" if ln in marks else "folded")
        res[ln] = (key, flags, ln in errs)
    return res


# The options a declaration's own attributes decide. The others (-Wstringop-*, -Wformat-overflow=, -Warray-bounds,
# -Wrestrict, ...) are the middle end's: they also read the `access` and `restrict` information of a declaration
# and the object-size builtins, so they are reported as open, not compared.
DECIDED = {"-Wnonnull", "-Wunused-result", "-Wdeprecated-declarations", "-Wformat=", "-Wformat", "-Wformat-nonliteral",
           "-Wformat-security", "-Wformat-extra-args", "-Wformat-zero-length", "-Wformat-y2k", "-Wunused-value", "marker", "folded", "error"}


# Known differences, not slice 19's: the function (or ctype `is*` function) and why. They are printed, not counted.
OPEN = {
    "gettext": "the compiler's own builtin knowledge of the function adds -Wnonnull to a plain declaration",
    "dgettext": "the compiler's own builtin knowledge of the function adds -Wnonnull to a plain declaration",
    "btowc": "the reference's calls are merged by the compiler's own knowledge of the function; no attribute decides it",
    "wctob": "the reference's calls are merged by the compiler's own knowledge of the function; no attribute decides it",
    "fwrite_unlocked": "libniji's declaration is the plain function, so the compiler's own builtin knowledge adds -Wnonnull",
}
# Known differences the `e` test explains (decided per line in `check`, printed, not counted).
ERRNO_VETO = ("the call may set errno (the reference's objects keep an errno check after it at -O2), so libniji's "
              "declaration is not pure, and the compiler does not report a call whose result is unused (-Wunused-value)")
ERRNO_INLINE = ("the reference's caller folds the errno check with no pure or const in the table: its header expands the "
                "call inline; libniji's calls the library, which keeps the check (not slice 19's)")
CTYPE_OPEN = re.compile(r"^is(alnum|alpha|blank|cntrl|digit|graph|lower|print|punct|space|upper|xdigit)$")


def open_reason(name):
    if CTYPE_OPEN.match(name):
        return "slice 6: libniji's ctype.h macro for the function names _IS* enumerators it does not declare"
    return OPEN.get(name)


def decided(flags):
    return {f for f in flags if f in DECIDED}


def fmt_kind(n):
    if "scanf" in n:
        return "scanf"
    if "strftime" in n:
        return "strftime"
    if "strfmon" in n:
        return "strfmon"
    return "printf"


# C 7.22.6.2: div, ldiv, lldiv and imaxdiv compute from their arguments alone (the compiler does not merge two calls
# that return a structure, so the clobber test cannot tell): const by the standard's own definition.
CONST_BY_STANDARD = {"div", "ldiv", "lldiv", "imaxdiv"}

# strndup's result size is at most n + 1: the compiler's own knowledge of the function answers the alloc_size test.
BUILTIN_SIZE = {"strndup", "strdup"}


def derive(res, pr, names):
    rows = {n: {"nonnull": [], "wur": "", "noreturn": "", "format": "", "malloc": "", "alloc_size": "", "deprecated": "", "sentinel": "", "pure": ""} for n in names}
    fmt = {}
    conflict = set()
    for ln, (key, flags, err) in sorted(res.items()):
        n, kind = key[0], key[1]
        r = rows[n]
        if kind == "bp":
            if err or flags & {"-Wattributes", "error"}:
                conflict.add(n)
            continue
        if err:
            continue
        if kind == "r":
            if "-Wunused-result" in flags:
                r["wur"] = "1"
            if "-Wdeprecated-declarations" in flags:
                r["deprecated"] = "1"
            if "-Wunused-value" in flags:
                r["pure"] = "1"
            if "-Wformat=" in flags and pr[n].rstrip().endswith("...)"):
                r["sentinel"] = "?"
        elif kind == "n" and "-Wnonnull" in flags:
            r["nonnull"].append(str(key[2]))
        elif kind == "s" and r["sentinel"]:
            if "-Wformat=" not in flags and r["sentinel"] == "?":
                r["sentinel"] = "%d" % (key[2] - 1)
        elif kind == "h" and "-Wformat=" in flags:
            fmt.setdefault(n, key[2])
        elif kind == "f" and flags & {"-Wformat-nonliteral", "-Wformat-security"}:
            fmt[n] = key[2]
        elif kind == "g" and "-Wformat" in flags and "sentinel" not in r:
            pass
        elif kind == "nr" and "marker" not in flags:
            r["noreturn"] = "1"
        elif kind == "ct" and "marker" not in flags and r["pure"]:
            r["pure"] = "const"
        elif kind == "ma" and "marker" not in flags:
            r["malloc"] = "1"
        elif kind == "as" and "marker" not in flags and n not in BUILTIN_SIZE:
            r["alloc_size"] = key[2]
    for r in rows.values():
        if r["sentinel"] == "?":
            r["sentinel"] = ""
    for n, j in fmt.items():
        _, ps = split_params(pr[n], n)
        variadic = ps[-1] == "..."
        first = len([p for p in ps if p != "..."]) + 1 if variadic else 0
        rows[n]["format"] = "%s:%d:%d" % (fmt_kind(n), j, first)
    for n in conflict:
        if rows[n]["pure"]:
            rows[n]["pure"] = "const"
    for n in CONST_BY_STANDARD & set(rows):
        if rows[n]["pure"]:
            rows[n]["pure"] = "const"
    # The errno veto, last: a caller of F keeps an errno check after the call in the reference's objects.
    for n in errno_kept(res):
        if n in rows:
            rows[n]["pure"] = ""
    return rows


def errno_kept(res):
    """The functions whose `e` test kept its marker: the call may set errno, so neither pure nor const."""
    return {key[0] for key, flags, err in res.values() if key[1] == "e" and not err and "marker" in flags}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("mode", choices=["table", "check"])
    ap.add_argument("--include", default=os.path.join(ROOT, "libc-include"))
    ap.add_argument("--out", default=os.path.join(ABI, "diagattrs.tsv"))
    ap.add_argument("--keep", default="")
    args = ap.parse_args()
    pr = protos()
    names, heads = declared(args.include, pr)
    text, meta = corpus(names, heads, pr)
    ref = results(text, meta, None, args.keep, "ref")
    if args.mode == "table":
        bad = sorted({ref[ln][0][0] for ln in ref if ref[ln][2]})
        if bad:
            print("attr_probe: %d functions have a test that did not compile on the reference side: %s" % (len(bad), " ".join(bad[:40])), file=sys.stderr)
        rows = derive(ref, pr, names)
        with open(args.out, "w") as f:
            f.write("name\tnonnull\twur\tnoreturn\tformat\tmalloc\talloc_size\tdeprecated\tsentinel\tpure\n")
            for n in names:
                r = rows[n]
                nn = ",".join(sorted(set(r["nonnull"]), key=int))
                if nn or any(r[k] for k in ("wur", "noreturn", "format", "malloc", "alloc_size", "deprecated", "sentinel", "pure")):
                    f.write("\t".join([n, nn, r["wur"], r["noreturn"], r["format"], r["malloc"], r["alloc_size"], r["deprecated"], r["sentinel"], r["pure"]]) + "\n")
        print("attr_probe: %d functions probed, table written to %s" % (len(names), args.out))
        return
    got = results(text, meta, args.include, args.keep, "niji")
    diff = 0
    opened = {}
    known = {}
    vetoed = errno_kept(ref)
    table = derive(ref, pr, names)
    for ln in sorted(ref):
        if ref[ln][0][1] == "bp":
            continue  # a table-only test (is the compiler's own knowledge of the function const?), not a compared line
        for f in ref[ln][1] ^ got[ln][1]:
            if f not in DECIDED and f != "note":
                opened[f] = opened.get(f, 0) + 1
        if decided(ref[ln][1]) != decided(got[ln][1]) or ref[ln][2] != got[ln][2]:
            why = open_reason(ref[ln][0][0])
            n, kind = ref[ln][0][0], ref[ln][0][1]
            if not why and ref[ln][2] == got[ln][2]:
                only = decided(ref[ln][1]) ^ decided(got[ln][1])
                if n in vetoed and only == {"-Wunused-value"}:
                    why = ERRNO_VETO
                elif kind == "e" and "folded" in ref[ln][1] and "marker" in got[ln][1] and not table[n]["pure"]:
                    why = ERRNO_INLINE
            if why:
                known[ref[ln][0][0]] = why
                continue
            diff += 1
            print("line %d %s: reference %s%s, libniji %s%s" % (ln, " ".join(map(str, ref[ln][0])), sorted(ref[ln][1]), " ERR" if ref[ln][2] else "", sorted(got[ln][1]), " ERR" if got[ln][2] else ""))
    for n, why in sorted(known.items()):
        print("attr_probe: known difference, %s: %s" % (n, why))
    print("attr_probe: open (not decided by attributes, not compared): %s" % (", ".join("%s x%d" % kv for kv in sorted(opened.items())) or "none"))
    print("attr_probe: %d functions, %d tests, %d differ" % (len(names), len(ref), diff))
    sys.exit(1 if diff else 0)


if __name__ == "__main__":
    main()
