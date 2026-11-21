# HEAP-ADDRESS-SPACE-9 report

Branch `task/heap-address-space-9` (from -8's head `fe1ac3a78`); commits: `d3edbb094` + `60ec2a659` (test, ruling
#7001), `619c10668` (plants), handoff/report on top. No library code change. Host: Zen 1 build host (role keke r2).

## Premise

Ruling #6997: run -8's cbench_startup was slower after (startup_kinstr_c +7.0k..7.9k kernel instructions, user rows
+6..+28); find the cause (strace and faults of the C start-up child, before vs after) and fix it, or hand back the
numbers for a ruling if it is the inherent cost of mapping segments by use.

## Finding: an extra PT_LOAD from a layout boundary, not the heap mapping

Static `cbench_startup` linked as `cargo xtask bench` does, against next `3f2a84520`'s release products (the primed
base tree) and the head's (`NIJI_FULL_PROFILE=1 cargo xtask so`, `heap-address-space-9-relso`):

- `true` child: strace identical (no heap call: execve, arch_prctl x2, one 67104-byte mapping, set_tid_address,
  set_robust_list, rseq, rt_sigaction, mprotect 28672, exit_group); page faults 8-9 in both.
- Sections: `.rodata` 0x922dc in both; `.text` 0x2ecc27 -> 0x2ed0e7 (+0x4c0). Next: `.rodata` 0x6fd000..0x78f2dc,
  `.rodata1` (64 KiB-aligned) at 0x790000, gap 0xd24, one read-only LOAD (4 LOADs). Head: `.rodata` starts a page
  later and ends 0x7902dc, `.rodata1` at 0x7a0000, gap ~60 KiB, ld splits the read-only LOAD (5 LOADs).
- Counterfactual (101 runs each, `perf stat` instructions:k / instructions:u, medians):

| link | LOADs | kernel instr | user instr |
|---|---|---|---|
| next | 4 | 165.7k, 164.3k (repeat) | 2190 |
| next + 4 KiB text pad | 5 | 173.4k | 2198 |
| head | 5 | 172.3k, 173.5k (repeat) | 2198 |
| head + 60 KiB text pad | 4 | 166.7k | 2190 |

  So the cbench_startup true-row regression (kernel +7.5k, user +8) is entirely the extra LOAD; next is on the lucky
  side of the boundary by 0xd24 bytes, and any change adding >= 985 bytes of text to this link pays it.
- Heap's own start-up cost, LOAD count equal (next vs head + pad, children with LC_ALL=en_US.UTF-8, which malloc):
  user min setlocale 61840 -> 61846, printf 65449 -> 65453, wctype 62309 -> 62315 (+4..+6, the first reservation's
  arithmetic); kernel min 421.4k -> 421.9k, 435.1k -> 436.8k, 514.8k -> 512.8k (medians +1.7k, -2.1k, -1.7k: noise);
  faults equal (38 / 41-42 / 48); wctype system calls the same (mmap, munmap, munmap, mbind; sizes differ: the trims
  13.9 + 15.2 MiB, was 2.5 + 14.2 MiB; mbind 4.3 MiB, was 16.1 MiB).

## RULING NEEDED

- cbench_startup on this branch: the delta is the static link's read-only LOAD split (layout jitter at `.rodata1`'s
  64 KiB boundary), not the heap. Recommend: land as the partial fix for #21 with the row recorded as layout jitter, and
  cut a layout story that removes the split for every static program (most links already have it: the split happens
  whenever `.rodata` ends more than a page below the next 64 KiB boundary). Alternative: trim >= 232 bytes of heap text
  so this one link stays below the boundary (luck for one binary; not recommended).

## Ruling #7001: the RLIMIT_AS test

`kernel_pages_map_aligned_under_an_address_space_limit` (crates/boundary/tests/s03_memory_symbols.rs, re-executed
child): limit = size + 20 MiB; precondition: a 32 MiB mapping is refused; `map(SEG, SEG)` and `map_room(8 KiB, 4 MiB,
SEG, SEG)` return aligned, zero-filled, writable memory; `grow` maps the free upper half of its own mapping and refuses
the taken lower half. Plants (handoff/HEAP-ADDRESS-SPACE-9-plants.txt): map's exact fallback gone, map_room's head not
mapped, grow replacing a mapping.

## Jobs (keke)

- `heap-address-space-9-s0` (step 0, inherited head): rc 0. `-relso` (release products of the head): rc 0.
- Probes: `-pbase`, `-phead`, `-pad` (superseded: the pad object lacked .note.GNU-stack), `-pad2`, `-modes`.
- `-t1`: rc 1 (clippy: panic!/expect in the new test), fixed in `60ec2a659`. Final head: `-t2` rc 0 (xtask clippy,
  cargo clippy --workspace --all-targets --all-features -D warnings, static, the test); `-plants` rc 0: 2 CAUGHT,
  1 WARN-NO-MSG (grow-replaces: caught by the new test, but at its next assertion, "grow: a free range that fits the
  limit is mapped", not "a taken range is never replaced"; why the replacing flags made both grows fail under the
  limit was not investigated); `-fullhot` rc 0 (30 passed; release counts: dynamic start-up 64 (bound 67),
  startup-only 10, coreutils-shaped 45, setlocale + printf 43, all at or under their bounds).
- Still running at hand-back (queue waits of 10-20 min per job on the build host): `-xtest` (`cargo xtask test`,
  products built, 155 of 6910 tests passed at 16:20Z). Not run: the Zen 1 fast-profile dynamic start-up count for
  DYN_TRUE_BOUND [73, 67] (#6998, "if cheap": it was queued behind `-xtest` and stopped at hand-back; the release
  count on Zen 1 is 64, bound 67, in `-fullhot`).
- Not run: precheck (the role stays busy with the chain; the hand-back is INCOMPLETE pending the ruling).
  `cargo xtask affected` escalated to FULL on run -8 (entry.S, xtask): FULL: the train's.
