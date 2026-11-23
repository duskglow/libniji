#!/usr/bin/env python3
"""STARTUP-REPROFILE: phase table from incl.py fn.tsv files (inclusive counts of disjoint subtrees).
usage: phase.py A.fn.tsv [B.fn.tsv ...]"""
import sys

P = "niji_ld::reloc::"
REL = "<niji_ld::arch::X86_64, niji_boundary::ld::image::ProcImage>"
rows = [
    ("ld.so self-relocation (relocate_self + publish)", ["niji_boundary::bootstrap::relocate_self", "niji_boundary::bootstrap::publish_dynamic"]),
    ("symbolic relocations (reloc::one: decode, lookup, version origin, store)", [P + "one::" + REL]),
    ("RELATIVE relocations and table walk (pass1 less one)", [P + "pass1::" + REL, "-" + P + "one::" + REL]),
    ("per-object tables (Object::with_style = Object::new)", ["<niji_ld::symbols::Object>::with_style", "<niji_ld::symbols::Object>::new"]),
    ("  of which GnuHash::symbol_count", ["<niji_ld::elf::GnuHash>::symbol_count"]),
    ("whole-file image view and span checks (image_view)", ["niji_boundary::ld::image::image_view"]),
    ("version requirements (check_versions)", [P + "check_versions"]),
    ("search and open (search, open_candidate)", ["niji_ld::search::search::<niji_ld::arch::X86_64>", "niji_boundary::ld::image::open_candidate"]),
    ("mapping (map_planned)", ["niji_ld::map::map_planned::<niji_boundary::ld::image::StartMapper>"]),
    ("RELRO (protect_relro)", [P + "protect_relro::<niji_boundary::ld::image::ProcImage>"]),
    ("debugger rendezvous, publish dynamic", ["niji_boundary::ld::link::rendezvous_start", P + "publish_dynamic::<niji_boundary::ld::image::ProcImage>"]),
    ("init hooks and running initialisers", ["niji_ld::init::hooks", "niji_boundary::ld::initfini::run_init"]),
    ("libc start, main, exit (_start)", ["_start"]),
]


def load(p):
    d, tot = {}, 0
    for line in open(p):
        if line.startswith("#"):
            tot = int(line.split()[-1])
            continue
        i, e, s = line.rstrip("\n").split("\t")
        d[s] = (int(i), int(e))
    return d, tot


tabs = [load(p) for p in sys.argv[1:]]
listed = [0] * len(tabs)
for name, keys in rows:
    vals = []
    for t, (d, tot) in enumerate(tabs):
        v = 0
        for k in keys:
            sign = -1 if k.startswith("-") else 1
            v += sign * d.get(k.lstrip("-"), (0, 0))[0]
        if not name.startswith("  ") and name != "libc start, main, exit (_start)":
            listed[t] += v
        vals.append(v)
    print(f"| {name} | " + " | ".join(f"{v:,} ({100.0 * v / tabs[i][1]:.1f}%)" for i, v in enumerate(vals)) + " |")
# rest of the loader: everything under niji_machine_ld_entry not in the rows above (link's own code, entry, env, CPU
# features, TLS, loader-export lookups, small helpers)
rest = []
for t, (d, tot) in enumerate(tabs):
    ld = d["niji_machine_ld_entry"][0] - d.get("_start", (0, 0))[0]
    rest.append(ld - listed[t])
print("| rest of the loader (link and entry code, env, TLS, loader exports, helpers) | " + " | ".join(f"{v:,} ({100.0 * v / tabs[i][1]:.1f}%)" for i, v in enumerate(rest)) + " |")
print("| total (PT samples) | " + " | ".join(f"{tot:,}" for d, tot in tabs) + " |")
for k in ["<niji_ld::elf::Elf>::read_phdr", "<niji_ld::elf::Elf>::image_span_kind", "<niji_ld::elf::Dynamic>::once",
          "<niji_ld::elf::Verdefs as core::iter::traits::iterator::Iterator>::next", "<niji_ld::elf::Dynamic>::hashed_string",
          "niji_ld::symbols::resolve", "<niji_ld::elf::Elf>::check_table", "<niji_ld::elf::Elf>::parse::<niji_ld::arch::X86_64>"]:
    print(f"| cross: {k} incl | " + " | ".join(f"{d.get(k, (0, 0))[0]:,}" for d, tot in tabs) + " |")
