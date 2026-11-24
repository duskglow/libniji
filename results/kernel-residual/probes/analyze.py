#!/usr/bin/env python3
"""KERNEL-RESIDUAL-PROFILE: attribute the kernel samples krp.py recorded (record.json, stat.json in OUTDIR).

  python3 analyze.py OUTDIR [--runs N]

Two views per program and event, each sample weighted by its period, per run, scaled so that the
non-overhead samples sum to the perf stat median (the samples of perf's own mmap/exec bookkeeping, which perf
stat does not pay, are dropped first):
  entry      where the kernel was entered (syscall by name, page fault by kind, exit_group, execve, irq), with
             sub-buckets for munmap, mmap and exit_group;
  mechanism  what the kernel did, by the deepest frame that names a mechanism (PTE fill of file pages, PTE
             teardown, page zeroing, copying, page allocation and freeing, page-table pages, VMA tree work,
             TLB flushes, memory-cgroup accounting).
"""
import json
import sys

OUT = sys.argv[1]
RUNS = int(sys.argv[sys.argv.index("--runs") + 1]) if "--runs" in sys.argv else 300
# --shared: the files of krp.py --only shared; --pair A,B: the two variants compared (B - A).
SUFFIX = "_shared" if "--shared" in sys.argv else ""
A, B = (sys.argv[sys.argv.index("--pair") + 1] if "--pair" in sys.argv else "ref,niji").split(",")

OVERHEAD = ("perf_event_mmap", "perf_iterate", "perf_event_exec", "perf_event_comm", "perf_event_task",
            "perf_event_namespaces", "perf_event_fork")


def has(chain, names):
    return any(f in names or f.startswith(names) for f in chain)


def fault_kind(ch):
    if has(ch, ("do_cow_fault",)):
        return "private-file-write"
    if has(ch, ("wp_page_copy", "do_wp_page", "__wp_page_copy_user", "wp_page_reuse")):
        return "copy-on-write"
    if has(ch, ("filemap_map_pages", "do_read_fault", "filemap_fault", "next_uptodate_folio", "filemap_get_entry")):
        return "file-read"
    if has(ch, ("do_anonymous_page", "alloc_anon_folio", "vma_alloc_zeroed_movable_folio", "folio_add_new_anon_rmap")):
        return "zero-fill"
    if has(ch, ("expand_stack", "expand_downwards")):
        return "stack-expand"
    if has(ch, ("__pte_alloc", "pte_alloc_one", "__pmd_alloc", "__pud_alloc")):
        return "page-table-alloc"
    return "unclassified"


def sub_munmap(ch):
    if has(ch, ("free_pgtables", "free_pgd_range", "___pte_free_tlb")):
        return "page-table free"
    if has(ch, ("unmap_page_range", "zap_pte_range", "unmap_vmas", "unmap_single_vma")):
        return "PTE teardown"
    if has(ch, ("tlb_finish_mmu", "flush_tlb_mm_range", "tlb_flush_mmu", "release_pages", "free_pages_and_swap_cache")):
        return "TLB flush / page release"
    return "VMA work"


def entry(ch):
    if not ch:
        return "none"
    if has(ch, OVERHEAD):
        return "OVERHEAD"
    r = ch[0]
    if r.startswith(("asm_sysvec", "asm_common_interrupt", "sysvec")) or "interrupt" in r:
        return "irq"
    sysname = next((f[len("__x64_sys_"):] for f in ch if f.startswith("__x64_sys_")), None)
    fault = "asm_exc_page_fault" in ch
    if sysname == "exit_group":
        return "exit_group: " + (sub_munmap(ch) if "exit_mmap" in ch else "other")
    if sysname == "munmap":
        return "munmap: " + sub_munmap(ch)
    if sysname == "mmap":
        if has(ch, ("do_vmi_munmap", "do_vmi_align_munmap")):
            return "mmap: unmap of the range replaced (MAP_FIXED)"
        return "mmap: new mapping"
    if sysname:
        return f"sys:{sysname}" + (" (fault inside)" if fault else "")
    if fault:
        return "fault: " + fault_kind(ch)
    if r.startswith(("entry_SYSCALL", "do_syscall")):
        return "syscall entry/exit"
    return "other"


MECH = [
    ("page zeroing", ("clear_page_erms", "clear_page_rep", "clear_page_orig", "clear_user")),
    ("page copy", ("copy_page", "copy_mc_", "__wp_page_copy_user")),
    ("TLB flush", ("flush_tlb_mm_range", "native_flush_tlb", "flush_tlb_func", "smp_call_function_many_cond",
                   "flush_tlb_one", "invlpg")),
    ("page-table pages", ("__pte_alloc", "pte_alloc_one", "__pmd_alloc", "__pud_alloc", "free_pgtables",
                          "free_pgd_range", "___pte_free_tlb", "pte_free")),
    ("memcg / stat accounting", ("__mod_lruvec_state", "__mod_memcg_lruvec_state", "__lruvec_stat_mod_folio",
                                 "__mod_node_page_state", "mem_cgroup_", "__count_memcg_events", "__mod_lruvec_page_state")),
    ("PTE fill, file pages (fault-around)", ("filemap_map_pages", "set_pte_range", "folio_add_file_rmap_ptes",
                                             "next_uptodate_folio", "filemap_map_folio_range")),
    ("PTE teardown", ("zap_pte_range", "unmap_page_range", "folio_remove_rmap_ptes", "unmap_vmas")),
    ("page alloc / free", ("__alloc_pages", "get_page_from_freelist", "rmqueue", "free_unref_page", "release_pages",
                           "free_pages_and_swap_cache", "__folio_put", "free_unref_folios", "__free_pages")),
    ("VMA tree / VMA objects", ("mas_", "mtree_", "vma_merge", "__split_vma", "vm_area_", "remove_vma", "vma_complete",
                                "vma_prepare", "mmap_region", "unmap_region", "vma_link", "vma_expand", "vms_",
                                "get_unmapped_area", "arch_get_unmapped_area", "vm_unmapped_area", "kmem_cache_",
                                "find_vma", "vma_iter")),
    ("exec / ELF load", ("load_elf_binary", "bprm_", "begin_new_exec", "setup_arg_pages", "do_open_execat")),
    ("file open / lookup", ("path_openat", "link_path_walk", "do_filp_open", "filename_lookup", "__fput", "vfs_")),
]


def mechanism(ch):
    if has(ch, OVERHEAD):
        return "OVERHEAD"
    for f in reversed(ch):
        for name, keys in MECH:
            if f in keys or f.startswith(keys):
                return name
    if "asm_exc_page_fault" in ch:
        return "fault entry / other fault work"
    if any(f.startswith("__x64_sys_") for f in ch):
        return "syscall other"
    return "other"


def views(chains):
    e, m = {}, {}
    for s, w in chains.items():
        ch = s.split(";") if s else []
        k = entry(ch)
        e[k] = e.get(k, 0) + w
        k = mechanism(ch)
        m[k] = m.get(k, 0) + w
    return e, m


def detail(agg, stat, ev, name, view, key, depth):
    """Samples of one entry or mechanism bucket: by entry (for a mechanism) and by their deepest frames."""
    for kit in (A, B):
        ch_all = agg[f"{ev}|{name}|{kit}"]["chains"]
        tot = sum(w for s, w in ch_all.items() if not has(s.split(";"), OVERHEAD)) / RUNS
        scale = stat[f"{name}.{kit}"][ev] / tot
        by_entry, by_tail = {}, {}
        for s, w in ch_all.items():
            ch = s.split(";") if s else []
            if (entry(ch) if view == "entry" else mechanism(ch)) != key:
                continue
            e = entry(ch)
            by_entry[e] = by_entry.get(e, 0) + w
            t = " < ".join(reversed(ch[-depth:]))
            by_tail[t] = by_tail.get(t, 0) + w
        print(f"\n   {kit} {view} '{key}': {sum(by_entry.values()) / RUNS * scale:,.0f} per run")
        if view != "entry":
            for k, v in sorted(by_entry.items(), key=lambda x: -x[1])[:8]:
                print(f"     by entry {k[:50]:50} {v / RUNS * scale:9,.0f}")
        for k, v in sorted(by_tail.items(), key=lambda x: -x[1])[:10]:
            print(f"     {v / RUNS * scale:9,.0f}  {k[:150]}")


def main():
    agg = json.load(open(f"{OUT}/record{SUFFIX}.json"))
    if "--detail" in sys.argv:
        i = sys.argv.index("--detail")
        ev, name, view, key, depth = sys.argv[i + 1:i + 6]
        detail(agg, json.load(open(f"{OUT}/stat{SUFFIX}.json")), ev, name, view, key, int(depth))
        return
    stat = json.load(open(f"{OUT}/stat{SUFFIX}.json"))
    for ev in ("instructions:k", "cycles:k"):
        for name in ("empty", "true", "ls", "sed"):
            res = {}
            for kit in (A, B):
                e, m = views(agg[f"{ev}|{name}|{kit}"]["chains"])
                over = e.get("OVERHEAD", 0) / RUNS
                tot = sum(e.values()) / RUNS - over
                scale = stat[f"{name}.{kit}"][ev] / tot
                res[kit] = ({k: v / RUNS * scale for k, v in e.items() if k != "OVERHEAD"},
                            {k: v / RUNS * scale for k, v in m.items() if k != "OVERHEAD"}, over, scale)
            a, b = res[A], res[B]
            d = stat[f"{name}.{B}"][ev] - stat[f"{name}.{A}"][ev]
            print(f"\n-- {ev} {name}: perf stat {A} {stat[f'{name}.{A}'][ev]:,.0f} {B} {stat[f'{name}.{B}'][ev]:,.0f}"
                  f" diff {d:+,.0f}; scale {a[3]:.3f} / {b[3]:.3f} (perf overhead dropped {a[2]:,.0f} / {b[2]:,.0f})")
            for title, i in (("entry", 0), ("mechanism", 1)):
                keys = sorted(set(a[i]) | set(b[i]), key=lambda k: -abs(b[i].get(k, 0) - a[i].get(k, 0)))
                print(f"   {title}:")
                for k in keys[:14 if i == 0 else 12]:
                    x, y = a[i].get(k, 0), b[i].get(k, 0)
                    print(f"     {k[:46]:46} {x:10,.0f} {y:10,.0f} {y - x:+10,.0f}  {100 * (y - x) / d:+5.0f}%")


main()
