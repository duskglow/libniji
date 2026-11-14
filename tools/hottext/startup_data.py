#!/usr/bin/env python3
"""STARTUP-DATA-WINDOWS (D-X.50): the data objects start-up may touch, declared by rule, and the check that
`page_faults true` touches no other.

usage: startup_data.py <linked probe> <isolated probe> <startup-text.keys> <profile> <startup-data.keys> <probe object>

<linked probe>: crates/core/tests/oracle/page_faults.c linked statically against the products as
crates/core/tests/hot_text_products.rs links it, from <probe object> (page_faults.c compiled alone;
tools/hottext/startup_data.sh does both).
<isolated probe>: the same link with every input section of `.data`, `.data1`, `.bss` and `.lbss` on a
64 KiB boundary of its own (the default linker script with SUBALIGN(65536) on those output sections).

LAYOUT-CANONICAL-SET (ruling R3 of plan/layout-budget-rethink.md): the written set is DECLARED BY RULE, from the
linked probe alone, so every build host writes the same list for the same tree (the start-up text keys are
already the union of the CPU classes, LT-4). The declared set is the data objects addressed by
  1. the operands of the start-up functions (the keys of <startup-text.keys> for <profile>): the RIP-relative
     ones (`objdump -d` prints the target as `# <addr> <sym>`) and the absolute ones (`$0x<addr>` or a bare
     `0x<addr>` displacement: the linker relaxes a GOT load of a static non-PIE link to `mov $<addr>, %reg`,
     so `environ`, `process::ARGC` and the CPU-classed words of csys/src/machine are reached that way);
     every CPU class's path is in the text keys, so its data is declared on every host;
  2. the same operands of the probe program's own functions (the text symbols of <probe object>: `main`
     prints on `stderr`), the probe's own statics left out;
  3. one level of pointers: every aligned 8-byte word inside an object of 1 or 2 that holds the address of
     a data object (a static non-PIE link has resolved them; `locale::C_SET` points at the C locale's tables).
Left out: text, TLS, unnamed bytes, compiler-numbered constants (`anon.<hash>.<n>`: no build-stable name;
the layout check counts those through the start-up functions' operands at check time), and objects in
`.lbss`: the build moves there the large objects most processes never or barely touch (D-X.22), and their
head would count a written page of the STARTUP-DATA-WINDOWS contract that start-up does not write
(`global_locale::SLOTS`, written by the locale paths only).

The check (the rule's proof on the build host): the page faults of `<probe> true` with their data addresses
(`perf record -e page-faults -d`), and those of `<isolated probe> true`, writable sections only (no two
sections share a page there, so every writable object start-up reads or writes faults at its own address:
LAYOUT-DATA-CLUSTERS-3, D-X.67), run under a fixed environment (`env -i` with LANG=C and a fixed PATH, and
`setarch -R`): every object they name must be declared. A traced object outside the declared set fails the
build, naming it and the host's CPU: the rule misses a dependence on that host (add a clause, never the object).
A host where perf cannot record (perf_event_paranoid > 2, no perf) fails: the check cannot run.
The kind is the section's: `rw` when the section is writable (.data, .bss, .data.rel.ro, ...), else `ro`.
Output (written fresh by every trace-at-build `cargo xtask so`, LT-5: nothing of an earlier build is kept):
  `<key> TAB <ro|rw> TAB <profile>` lines, keys build-stable (tools/hottext/keys.py), sorted.
"""
import os
import re
import shutil
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import keys as K  # noqa: E402

PROFILES = ["fast", "release"]
DATA_TYPES = set("dDbBrRvVgGsS")
# The trace environment: nothing of the caller's (a GLIBC_TUNABLES or LANG of the build host would run other paths).
TRACE_ENV = ["LANG=C", "PATH=/usr/bin:/bin"]
# Sections whose objects the rule leaves out (see the module doc).
COLD = (".lbss",)


def run(args, **kw):
    r = subprocess.run(args, capture_output=True, text=True, **kw)
    if r.returncode != 0:
        sys.exit(f"startup_data: {' '.join(args)} failed ({r.returncode}): {r.stderr.strip()[-400:]}")
    return r.stdout


def sections(exe):
    """[(start, end, name, writable, tls, code, file offset, progbits)] of the allocated sections of the probe."""
    out = []
    for line in run(["readelf", "-SW", exe]).splitlines():
        m = re.match(r"\s*\[\s*\d+\]\s+(\S+)\s+(\S+)\s+([0-9a-f]+)\s+([0-9a-f]+)\s+([0-9a-f]+)\s+\S+\s+(\S*)", line)
        # TLS is left out: a TLS symbol's value is a TLS-block offset, and .tbss overlaps the next section.
        if not m or "A" not in m.group(6) or "T" in m.group(6):
            continue
        start, size = int(m.group(3), 16), int(m.group(5), 16)
        flags = m.group(6)
        out.append((start, start + size, m.group(1), "W" in flags, "T" in flags, "X" in flags, int(m.group(4), 16), m.group(2) == "PROGBITS"))
    return out


def symbols(exe):
    """(data objects [(start, end, name)], functions {key: [(start, end)]}) of the probe."""
    data, funcs = [], {}
    for line in run(["nm", "-S", "--defined-only", exe]).splitlines():
        f = line.split()
        if len(f) == 4:
            a, s, t, n = f
        elif len(f) == 3:
            (a, t, n), s = f, "0"
        else:
            continue
        start, size = int(a, 16), int(s, 16)
        if t in DATA_TYPES:
            # The 4th field: nm gave a size (a linker marker such as __bss_start or _edata has none).
            data.append((start, start + max(size, 1), n, size > 0))
        elif t in "tTwWiI":
            funcs.setdefault(K.key(n), []).append((start, start + max(size, 1)))
    return sorted(data), funcs


def startup_fn_keys(path, profile):
    out = set()
    with open(path) as f:
        for line in f:
            p = line.rstrip("\n").split("\t")
            if line.startswith("#") or len(p) != 3:
                continue
            if profile in p[2].split(","):
                out.add(p[0])
    return out


def object_keys(obj):
    """(function keys, data keys) of what <obj> (an object file) defines."""
    fns, objs = set(), set()
    for line in run(["nm", "--defined-only", obj]).splitlines():
        f = line.split()
        if len(f) == 3 and f[1] in "tT":
            fns.add(K.key(f[2]))
        elif len(f) == 3 and f[1] in DATA_TYPES:
            objs.add(K.key(f[2]))
    return fns, objs


# An absolute address in an instruction's operands: `$0x<hex>` or a bare `0x<hex>` displacement, but not a
# RIP-relative displacement (`0x<hex>(%rip)`, whose target objdump prints after `#`).
ABSOLUTE = re.compile(r"(?<![\w$-])\$?0x([0-9a-f]+)\b(?!\(%rip\))")


def operand_targets(exe, ranges):
    """The data addresses the instructions inside `ranges` ([(start, end)]) name: RIP-relative and absolute."""
    out = []
    for lo, hi in sorted(ranges):
        text = run(["objdump", "-d", "--no-show-raw-insn", f"--start-address={lo:#x}", f"--stop-address={hi:#x}", exe])
        for line in text.splitlines():
            insn, _, note = line.partition("#")
            m = re.match(r"\s*([0-9a-f]+)\b", note)
            if m:
                out.append(int(m.group(1), 16))
            # `   4ebcc3:\tmov    $0x866db0,%rax`: the instruction follows the address's tab.
            parts = insn.split("\t")
            if len(parts) >= 2:
                out.extend(int(t, 16) for t in ABSOLUTE.findall(parts[-1]))
    return out


def pointer_targets(exe, secs, objs):
    """The aligned 8-byte words inside `objs` ([(start, end)]) read as addresses (file bytes; .bss has none)."""
    out = []
    with open(exe, "rb") as f:
        raw = f.read()
    for lo, hi in objs:
        sec = next((s for s in secs if s[0] <= lo < s[1] and s[7]), None)
        if sec is None:
            continue
        hi = min(hi, sec[1])
        base = sec[6] - sec[0]
        for a in range(lo + (-lo) % 8, hi - 7, 8):
            out.append(int.from_bytes(raw[base + a: base + a + 8], "little"))
    return out


def fault_addrs(exe):
    perf, setarch = shutil.which("perf"), shutil.which("setarch")
    if not perf or not setarch:
        sys.exit("startup_data: perf and setarch are needed to check the declared start-up data (perf record -e page-faults -d)")
    with tempfile.TemporaryDirectory() as d:
        data = os.path.join(d, "perf.data")
        run(["env", "-i", *TRACE_ENV, setarch, "-R", perf, "record", "-q", "-e", "page-faults", "-c", "1", "-d", "-o", data, "--", exe, "true"], cwd=d)
        text = run([perf, "script", "-F", "addr", "-i", data])
    return [int(t, 16) for t in text.split() if re.fullmatch(r"[0-9a-f]+", t)]


def resolve(addrs, data, secs, writable_only=False, skip_cold=False):
    """{key: (kind, (start, end))} of the data objects holding `addrs` (text, TLS and unnamed bytes skipped)."""
    out = {}
    for a in addrs:
        sec = next((s for s in secs if s[0] <= a < s[1]), None)
        if sec is None or sec[4] or sec[5] or (writable_only and not sec[3]) or (skip_cold and sec[2].startswith(COLD)):
            continue
        hit = [o for o in data if o[0] <= a < o[1]]
        if not hit:
            continue
        # The smallest object holding the address, by every name it has: a linker marker of no size
        # (__bss_start at the first .bss object) never stands in for the object, and each alias is
        # keyed (__progname and program_invocation_name: the move matches whichever names the section).
        sized = [o for o in hit if o[3]] or hit
        least = min(o[1] - o[0] for o in sized)
        for o in sized:
            if o[1] - o[0] != least or o[2].startswith(("anon.", ".L")):
                # A compiler-numbered constant has no build-stable name: the layout check finds it at check time
                # through the start-up functions' operands, which address it.
                continue
            out.setdefault(K.key(o[2]), ("rw" if sec[3] else "ro", (o[0], o[1])))
    return out


def declared(exe, fn_keys, data, funcs, secs):
    """The declared set (clauses 1-3 of the module doc): {key: (kind, (start, end))}."""
    ranges = [r for k in fn_keys for r in funcs.get(k, [])]
    direct = resolve(operand_targets(exe, ranges), data, secs, skip_cold=True)
    out = dict(direct)
    for k, v in resolve(pointer_targets(exe, secs, [v[1] for v in direct.values()]), data, secs, skip_cold=True).items():
        out.setdefault(k, v)
    return out


def cpu():
    with open("/proc/cpuinfo") as f:
        return next((ln.split(":", 1)[1].strip() for ln in f if ln.startswith("model name")), "unknown CPU")


def main():
    args = sys.argv[1:]
    if len(args) != 6 or args[3] not in PROFILES:
        sys.exit(__doc__)
    exe, iso, text_keys, profile, out_path, obj = args
    exe, iso = os.path.abspath(exe), os.path.abspath(iso)
    secs = sections(exe)
    data, funcs = symbols(exe)
    want = startup_fn_keys(text_keys, profile)
    if not any(k in funcs for k in want):
        sys.exit(f"startup_data: no function of {text_keys} ({profile}) in {exe}")
    probe, own = object_keys(obj)
    if not any(k in funcs for k in probe):
        sys.exit(f"startup_data: no function of {obj} in {exe}")
    # The probe's own statics (its function pointers, `header.0`) are not the library's: never declared.
    decl = {k: v for k, v in declared(exe, want | probe, data, funcs, secs).items() if k not in own}
    traced = set(resolve(fault_addrs(exe), data, secs))
    iso_data, _ = symbols(iso)
    traced |= set(resolve(fault_addrs(iso), iso_data, sections(iso), writable_only=True))
    extra = sorted(traced - set(decl) - own)
    if extra:
        sys.exit(f"startup_data: STARTUP-DATA-DECLARED: `page_faults true` touched data outside the declared set on {cpu()} ({profile}): "
                 f"{', '.join(extra)} (a dependence of this host the rule in tools/hottext/startup_data.py misses: add the clause, never the object)")
    with open(out_path + ".tmp", "w") as f:
        f.write("# Written by tools/hottext/startup_data.py during cargo xtask so (declared by rule from this build; never committed).\n")
        f.write("# STARTUP-DATA-WINDOWS (layout/contracts.tsv, D-X.50): the data objects start-up may touch (the operands of the\n")
        f.write("# start-up functions of startup-text.keys and of page_faults.c, one level of pointers); `page_faults true` touched no other.\n")
        f.write("# <key> TAB <ro|rw: the section's kind> TAB <profile>.\n")
        for k in sorted(decl):
            f.write(f"{k}\t{decl[k][0]}\t{profile}\n")
    os.replace(out_path + ".tmp", out_path)
    print(f"startup-data: {len(decl)} objects declared ({profile}: {sum(v[0] == 'ro' for v in decl.values())} ro); "
          f"the {len(traced)} traced on {cpu()} are among them; into {out_path}")


if __name__ == "__main__":
    main()
