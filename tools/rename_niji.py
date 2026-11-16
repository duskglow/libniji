#!/usr/bin/env python3
"""Rename the project from Liella to libniji (RENAME-NIJI-CODE, owner ruling 2026-11-16).

    python3 tools/rename_niji.py [--check | --self-test]

Run from the top of a git checkout. Renames the old name in the contents of every tracked file
outside the exclusions below, and in tracked paths (with `git mv`). Deterministic and
re-runnable: a second run changes nothing. `--check` changes nothing and exits 1 if a run would
change something (prints the files). `--self-test` checks the rules on fixed examples
(EXAMPLES, PATHS) and touches no file. No spec clause: a project rename, no behaviour.

Mapping, applied to each text in this order (the owner's table; identifier stem `niji`):

1. Kept verbatim (PROTECT): the author identity, the public repository URL (renamed by the owner
   after this lands), the internal ops names (Q6), and references to history that keeps the old
   name (handoff notes, the old repository's label).
2. Explicit spellings (EXPLICIT): fixed-size magic words keep their length; the native interface
   library becomes `libniji.so` / `-lniji`.
3. Rust v0/legacy mangled identifiers `<len>liella_x` become `<len-2>niji_x` (the length prefix
   counts the identifier, so a plain substitution would corrupt it).
4. `LIELLA` -> `NIJI` (version nodes, constants, macros, variables, `__LIELLA__`).
5. `Liella` -> `Niji` inside an identifier (`LiellaTm`), else `libniji` (prose, messages; owner Q1).
6. `liella` -> `niji` (crates, modules, paths, packages, run-time names, `+liella`).

Exclusions (EXCLUDE): history (handoff/, records/, validation/results/ and results/, the dated
measurement records, the release notes) and
the prose RENAME-NIJI-DOCS rewrites (docs/, README.md, plan/, spec/ prose, the top-level
documents). Binary files change only through equal-length EXPLICIT spellings.
"""

import os
import re
import subprocess
import sys

# Paths (prefixes) never rewritten.
EXCLUDE = (
    "handoff/",
    "records/",
    "validation/results/",
    "results/",
    "docs/",
    "plan/",
    "spec/behaviour/",
    "spec/security/",
    "spec/unicode/",
    "spec/ERRATA.md",
    "spec/clauses.tsv",
    "spec/schedule.tsv",
    "README.md",
    "RELEASE-NOTES.md",
    "AI_POLICY.md",
    "CLAUDE.md",
    "CODE_OF_CONDUCT.md",
    "CONTRIBUTING.md",
    "SECURITY.md",
    "implementation-plan.md",
    "glibc-functional-spec.md",
    "tools/rename_niji.py",
)

# Kept verbatim wherever they appear.
PROTECT = (
    "Liella Maintainers <liella@koiren.ai>",
    "liella@koiren.ai",
    "github.com/duskglow/liella",
    "liella-ops",
    "src/liella",
    "liella-historic",
    "LIELLA-UTILITIES-DESIGN",
)

# Explicit spellings, applied before the general rules (old, new).
EXPLICIT = (
    ("LIELLANC", "NIJI_NSC"),  # the name-service cache's 8-byte magic word keeps its size
    ('b"liella!!"', 'b"niji!!!!"'),  # an 8-byte test secret keeps its size
    ("libliella_native", "libniji"),
    ("-lliella_native", "-lniji"),
)

MANGLED = re.compile(r"(?<![0-9])([0-9]+)(liella)")
CAMEL = re.compile(r"Liella(?=[A-Za-z0-9_])")


def excluded(path):
    return any(path == p or (p.endswith("/") and path.startswith(p)) for p in EXCLUDE)


def mangled(text):
    out = []
    last = 0
    for m in MANGLED.finditer(text):
        n = int(m.group(1))
        ident = text[m.start(2):m.start(2) + n]
        if n >= 6 and len(ident) == n and re.fullmatch(r"[A-Za-z0-9_]+", ident):
            out.append(text[last:m.start()])
            out.append(str(n - 2) + "niji")
            last = m.end()
    out.append(text[last:])
    return "".join(out)


def rename_text(text):
    keep = {}
    for i, p in enumerate(PROTECT):
        token = "\0P%d\0" % i
        keep[token] = p
        text = text.replace(p, token)
    for old, new in EXPLICIT:
        text = text.replace(old, new)
    text = mangled(text)
    text = text.replace("LIELLA", "NIJI")
    text = CAMEL.sub("Niji", text)
    text = text.replace("Liella", "libniji")
    text = text.replace("liella", "niji")
    for token, p in keep.items():
        text = text.replace(token, p)
    return text


def rename_bytes(data):
    for old, new in EXPLICIT:
        if len(old) == len(new):
            data = data.replace(old.encode(), new.encode())
    return data


def rename_path(path):
    head, _, name = path.rpartition("/")
    parts = [rename_text(p) for p in head.split("/")] if head else []
    return "/".join(parts + [rename_text(name)])


def tracked():
    out = subprocess.run(["git", "ls-files", "-z"], check=True, capture_output=True).stdout
    return sorted(p for p in out.decode().split("\0") if p)


# The rules on fixed examples (`--self-test`; no spec clause: a project rename): input, output.
EXAMPLES = (
    ("liella-boundary liella_kernel::fd crate liella", "niji-boundary niji_kernel::fd crate niji"),
    ("LIELLA_NATIVE_1 LIELLA_COMPAT_LEGACY_DECLS __LIELLA__ -DLIELLA_PREFIX=/", "NIJI_NATIVE_1 NIJI_COMPAT_LEGACY_DECLS __NIJI__ -DNIJI_PREFIX=/"),
    ("Liella's loader; LiellaTm; \"Liella\"", "libniji's loader; NijiTm; \"libniji\""),
    ("_RNvNtCs1a_15liella_boundary3cpu4init 10liella_mem7copy_u8", "_RNvNtCs1a_13niji_boundary3cpu4init 8niji_mem7copy_u8"),
    ("x86_64 2.39+liella20261116.1-1 liella6-dev /usr/lib/liella", "x86_64 2.39+niji20261116.1-1 niji6-dev /usr/lib/niji"),
    ("-lliella_native libliella_native.so *b\"LIELLANC\"", "-lniji libniji.so *b\"NIJI_NSC\""),
    ("Liella Maintainers <liella@koiren.ai> github.com/duskglow/liella ~/src/liella-ops", "Liella Maintainers <liella@koiren.ai> github.com/duskglow/liella ~/src/liella-ops"),
    ("(liella-historic:handoff/x.md) handoff/LIELLA-UTILITIES-DESIGN.md", "(liella-historic:handoff/x.md) handoff/LIELLA-UTILITIES-DESIGN.md"),
)
PATHS = (
    ("include/liella/native.h", "include/niji/native.h"),
    ("tools/deb/liella-gcc.1", "tools/deb/niji-gcc.1"),
    ("handoff/liella.md", None),
    ("records/compat/COMPAT-1.toml", None),
    ("docs/x.md", None),
    ("results/x.md", None),
    ("crates/core/src/lib.rs", "crates/core/src/lib.rs"),
)


def self_test():
    bad = []
    for old, new in EXAMPLES:
        got = rename_text(old)
        if got != new:
            bad.append("rename_text(%r) = %r, want %r" % (old, got, new))
        if rename_text(new) != new:
            bad.append("rename_text is not idempotent on %r" % new)
    if rename_bytes(b"\0LIELLANC\0liella") != b"\0NIJI_NSC\0liella":
        bad.append("rename_bytes changes only equal-length explicit spellings")
    for path, new in PATHS:
        got = None if excluded(path) else rename_path(path)
        if got != new:
            bad.append("path %r -> %r, want %r" % (path, got, new))
    for line in bad:
        print("rename_niji: self-test FAIL " + line)
    print("rename_niji: self-test %s (%d examples, %d paths)" % ("FAIL" if bad else "PASS", len(EXAMPLES), len(PATHS)))
    return 1 if bad else 0


def main():
    if sys.argv[1:] == ["--self-test"]:
        return self_test()
    check = sys.argv[1:] == ["--check"]
    if sys.argv[1:] not in ([], ["--check"]):
        print(__doc__.strip().splitlines()[2].strip(), file=sys.stderr)
        return 2
    changed = []
    files = [p for p in tracked() if not excluded(p) and not os.path.islink(p) and os.path.isfile(p)]
    for path in files:
        with open(path, "rb") as f:
            data = f.read()
        try:
            new = rename_text(data.decode("utf-8")).encode("utf-8") if b"\0" not in data else rename_bytes(data)
        except UnicodeDecodeError:
            new = rename_bytes(data)
        if new != data:
            changed.append(path)
            if not check:
                with open(path, "wb") as f:
                    f.write(new)
    moves = [(p, rename_path(p)) for p in files if rename_path(p) != p]
    for old, new in moves:
        changed.append(old + " -> " + new)
        if not check:
            os.makedirs(os.path.dirname(new) or ".", exist_ok=True)
            subprocess.run(["git", "mv", old, new], check=True)
    for line in changed:
        print(("would change: " if check else "changed: ") + line)
    print("rename_niji: %d change(s)" % len(changed))
    return 1 if check and changed else 0


if __name__ == "__main__":
    sys.exit(main())
