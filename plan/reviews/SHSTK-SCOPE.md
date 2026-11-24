# SHSTK-SCOPE: what turning on the user shadow stack at run time would cost libniji, measured

Status: scoping study (planning story, 2026-11-22). No code lands with it. The prototype used for the
measurements is `handoff/SHSTK-SCOPE/prototype.patch` (not for landing). The bench sources and scripts are in
`handoff/SHSTK-SCOPE/bench/`, and the raw logs and median tables are in `handoff/SHSTK-SCOPE/`.

**In one sentence:** on the Intel N100 (Alder Lake-N), turning the shadow stack on costs each process one
system call, one page fault and 8-33 k kernel instructions at start-up (about 1-3 % of a dynamic start's
kernel work, 8 % of a static one's; under 0.05 ms). Each thread costs 13.3 k kernel instructions, one page
fault, one 4 KiB resident page and 8 MiB of reserved address space (create+join goes from 35.5 to 42.8 µs).
User-mode instructions do not change except in longjmp (+9), swapcontext (+12 per switch) and a C++ throw
through 10 frames (+45, 0.1 %).

## 1. Cost table (N100, kernel 6.8, `user_shstk`; prototype on vs off, same products)

How it was measured: `perf stat -r 50` (start-up) or `-r 5` (loops), three passes, median of the pass means.
Jobs: `shstk-scope-n100-m1` (one pass) and `shstk-scope-n100-m2` (two passes), both on the N100 host under
its timing queue. Builds: `shstk-scope-so` (products, Zen 1 build host). libniji ("n") is switched on by the
prototype's `NJSHSTK=` environment variable; the off arm sets `NJSHSTX=`, a string of the same length that
does not match. The reference C library (Ubuntu's 2.39 build, "ref") is switched on with
`GLIBC_TUNABLES=glibc.cpu.hwcaps=SHSTK` and off with `=-SHSTK`. Dynamic rows run as `env <vars> prog`
(libniji's libc is found through `LD_LIBRARY_PATH`, R14-3), so the cost of `env` itself is measured separately
and subtracted. Counts decide. Wall times are advisory.

### 1.1 Start-up (per process)

| row | arm | user instr | kernel instr | faults | syscalls | ms (advisory) |
|---|---|---:|---:|---:|---:|---:|
| n static empty main | off | 3,107 | 163,879 | 8 | 8 | 0.08 |
| n static empty main | on | 3,009 | 177,683 | 9 | 9 | 0.08 |
| n dynamic empty main | off | 295,834 | 834,491 | 65 | 28 | 0.41 |
| n dynamic empty main | on | 295,929 | 842,852 | 66 | 29 | 0.41 |
| n dynamic, three libraries | off | 415,230 | 1,274,720 | 77 | 73 | 0.56 |
| n dynamic, three libraries | on | 415,305 | 1,307,829 | 78 | 74 | 0.60 |
| ref static empty main | off / on | 154,478 / 154,617 | 297,548 / 316,983 | 27 / 27 | 14 / 17 | 0.17 / 0.18 |
| ref dynamic empty main | off / on | 117,801 / 118,165 | 590,971 / 607,062 | 51 / 52 | 29 / 32 | 0.30 / 0.29 |
| ref dynamic, three libraries | off / on | 154,738 / 155,140 | 936,597 / 948,500 | 67 / 68 | 62 / 65 | 0.46 / 0.47 |

The deltas (on minus off):

| row | user | kernel | kernel % | faults | syscalls |
|---|---:|---:|---:|---:|---:|
| n static | about 0 (-98, noise) | +13,804 | +8.4 % | +1 | +1 (`ARCH_SHSTK_ENABLE`) |
| n dynamic empty | +95 | +8,361 | +1.0 % | +1 | +1 |
| n dynamic, 3 libraries | +75 | +33,109 | +2.6 % | +1 | +1 |
| ref static | +139 | +19,435 | +6.5 % | 0 | +3 (enable, status, lock) |
| ref dynamic empty | +364 | +16,091 | +2.7 % | +1 | +3 |
| ref dynamic, 3 libraries | +402 | +11,903 | +1.3 % | +1 | +3 |

The kernel cost is the shadow stack's mapping (8 MiB reserved, one page touched). It does not depend on the
program's size. One delta is unexplained: libniji's three-library row adds +33 k against +8 k for the empty
row, and this held in all three passes. The reference shows no such growth. My hypothesis, not tested: the
prototype enables at ld.so's entry, before the libraries are mapped, so every later `mmap` searches around the
new shadow-stack area, while the reference enables after mapping. A real implementation enables later anyway
(section 3, item A). The next step is to re-measure with the enable moved to the program-entry transfer.

### 1.2 Per operation

Each value is (row at n=N minus row at n=0) / N.

| operation (N) | n off -> on user | n off -> on kernel | faults | n ns off -> on | ref user off -> on | ref kernel off -> on |
|---|---:|---:|---:|---:|---:|---:|
| pthread_create + join (2000) | 40,220 -> 40,219 | 44,300 -> 57,615 (+13.3 k) | +1 | 35,505 -> 42,790 | 2,276 -> 2,279 | 53,672 -> 66,842 (+13.2 k) |
| same, static program (2000) | 40,140 -> 40,141 | 44,891 -> 57,308 (+12.4 k) | +1 | 32,115 -> 39,255 | 2,179 -> 2,180 | 52,249 -> 66,994 |
| thread, pthread_exit from 5 frames, C cleanups (2000) | 132,235 -> 132,283 | 100,370 -> 114,964 | +1 | 72,665 -> 80,170 | 47,583 -> 47,596 | 53,740 -> 66,911 |
| same, `-fexceptions` (unwinder path) | 89,001 -> 89,047 | 44,910 -> 57,508 | +1 | 44,390 -> 52,750 | 37,340 -> 37,364 | 53,609 -> 66,932 |
| pthread_cancel of a thread in `pause` (2000) | 92,274 -> 92,282 | 97,503 -> 114,165 | +1 | 69,960 -> 78,290 | 16,923 -> 16,911 | 53,602 -> 66,937 |
| same, `-fexceptions` | 60,881 -> 60,896 | 51,014 -> 66,144 | +1 | 44,965 -> 54,630 | 15,093 -> 15,100 | 53,490 -> 67,689 |
| setjmp + longjmp, same frame (1e6) | 109 -> 118 (+9) | 0 | 0 | 15 -> 16 | 132 -> 157 (+25) | 0 |
| setjmp + longjmp out of 10 frames (1e6) | 164 -> 173 (+9) | 0 | 0 | 21 -> 23 | 187 -> 289 (+102) | 0 |
| swapcontext there and back (2 switches) (2e5) | 136 -> 160 (+24) | 811 -> 811 | 0 | 976 -> 1,063 | 126 -> 182 (+56) | 810 -> 810 |
| map_shadow_stack(64 KiB) + munmap: what makecontext adds once per context (1e4) | 67 | 20,938 | 1 | 6,143 | 38 | 21,532 |
| signal: raise + handler + sigreturn (1e5) | 85 -> 85 | 6,928 -> 9,128 (+2.2 k, +32 %) | 0 | 3,942 -> 4,514 | 84 -> 84 | 6,112 -> 8,261 (+2.15 k) |
| C++ throw-catch through 10 frames (1e5) | 39,166 -> 39,211 (+45) | 37 -> 37 | 0 | 5,522 -> 5,537 | 31,344 -> 31,374 (+29) | 31 -> 31 |
| vfork + _exit + waitpid (2000) | 161 -> 171 (+9) | 56,174 -> 56,714 (noise) | 0 | 30,440 -> 30,850 | 62 -> 67 | 55,896 -> 56,002 |

**Memory per thread.** Each thread gets its own shadow stack mapping. Measured with `/proc/self/smaps`, VmFlags
`ss`:

* libniji: 8,192 kB of address space per thread and for the main thread, 4 kB resident (one page holds 512
  return addresses). libniji's threads are created by `clone` without a size, so the kernel sizes the shadow
  stack from `RLIMIT_STACK` (capped at 4 GiB, kernel documentation), whatever the thread's own stack size is.
* Reference: the static thread got 8,196 kB, which is its stack plus guard. With `ulimit -s unlimited` the main
  thread got 4 GiB and a thread got 2,048 kB. So the reference passes the thread's stack size to the kernel
  (`clone3`).
* Under `ulimit -s unlimited` a libniji thread would get 4 GiB of address space each. I did not measure this
  for libniji; it follows from the kernel's rule. That runs out of the 128 TiB user address space at about
  32 k threads. Item F fixes it.

### 1.3 Real programs (subset in place of the workload kits)

The workload kits were not run. Their harness would need the prototype installed as the kit's C library, which
this study did not attempt. Instead, six Ubuntu programs ran through libniji's loader in program mode (the
prototype also enables there), off vs on, 5 runs x 3 passes:

| program | user instr off -> on | kernel instr off -> on | faults | ms off -> on |
|---|---:|---:|---:|---:|
| sort -n 200k lines | 513.82 M -> 513.82 M | 24.61 M -> 24.49 M | 3,768 -> 3,770 | 69.5 -> 69.5 |
| gzip -c | 339.87 M -> 339.87 M | 3.51 M -> 3.54 M | 288 -> 289 | 69.7 -> 69.9 |
| sed s/1/x/g | 379.18 M -> 379.18 M | 4.60 M -> 4.63 M | 246 -> 246 | 41.2 -> 41.5 |
| perl hash of 100k | 224.61 M -> 225.23 M | 24.81 M -> 24.84 M | 4,805 -> 4,806 | 86.1 -> 86.1 |
| python3 import json + one thread | 76.20 M -> 76.17 M | 13.57 M -> 13.62 M | 1,594 -> 1,597 | 31.6 -> 31.7 |
| find /usr/share/doc | 13.01 M -> 13.01 M | 32.80 M -> 32.61 M | 320 -> 319 | 15.7 -> 15.2 |

The difference is within the noise (±1 % kernel, user unchanged). Perl's +0.3 % user follows its hash seed,
which varies from run to run. Outside start-up and thread creation, the shadow stack costs these programs
nothing measurable.

## 2. The interface (kernel documentation, Intel SDM; the reference only as a black box)

**Kernel** (Documentation/arch/x86/shstk.rst at v6.8, and the UAPI `asm/prctl.h` and `asm-generic/mman-common.h`):

* `arch_prctl(ARCH_SHSTK_ENABLE 0x5001, ARCH_SHSTK_SHSTK 1)`: the kernel maps a shadow stack of
  min(RLIMIT_STACK, 4 GiB) and points SSP at its top. The function that makes the call can never return,
  because the new shadow stack holds no entry for it. `ARCH_SHSTK_DISABLE 0x5002`, `ARCH_SHSTK_LOCK 0x5003`
  (a mask; locked bits can no longer be changed except through ptrace's `ARCH_SHSTK_UNLOCK 0x5004`) and
  `ARCH_SHSTK_STATUS 0x5005` complete the set. Errors are EPERM (locked), ENOTSUPP and EINVAL. `ARCH_SHSTK_WRSS 2`
  lets user code write its shadow stack and is not wanted.
* Every operation is per thread. The state is inherited on clone, so enabling on the first thread before any
  other exists covers the process. **Disabling later only affects the calling thread.**
* Threads: the kernel allocates a shadow stack for each `clone(CLONE_VM)` child. The size is RLIMIT_STACK, or
  `clone3`'s `stack_size` when given. It is freed at thread exit. A `CLONE_VFORK` child runs on its parent's
  shadow stack.
* Signals: the kernel pushes a sigframe token (the old SSP with bit 63 set) and the restorer address. sigreturn
  checks and restores the old SSP. Nothing is needed from libc. libniji's own restorer passed the test.
* Fork: the shadow stack is copied like other memory. Exec: the features are cleared.
* `map_shadow_stack(453, addr, size, SHADOW_STACK_SET_TOKEN 1)` maps a new shadow stack with a restore token at
  its top. This is what makecontext needs.

**Hardware** (Intel SDM vol. 1 ch. 17 and vol. 2; AMD APM vol. 2 describes the same instructions):

* `call` pushes the return address to both stacks. `ret` pops both and raises #CP (SIGSEGV with the
  control-protection code) on a mismatch.
* `rdsspq` reads SSP. It is a no-op while shadow stacks are off, so code that zeroes the register first can
  test for "on" at no cost.
* `incsspq r` pops the low 8 bits of r entries (at most 255 per instruction).
* `rstorssp m64` switches SSP to the shadow stack whose restore token is at m64. `saveprevssp` then leaves a
  restore token for the old one.
* `wrss` and `wruss` write a shadow stack: privileged, or `ARCH_SHSTK_WRSS`.
* Indirect `jmp` and `call` need `endbr64` only under IBT, which Linux 6.8 does not enable for user space.

**The reference, black-box** (`/proc/self/status` `x86_Thread_features*` and the tunables' effects;
`handoff/SHSTK-SCOPE/bench/probe_ref*.sh`):

* With no tunable, the shadow stack is **off** even for a fully marked program, dynamic or static, on this
  kernel. `glibc.cpu.x86_shstk=on` or `=permissive` **alone does not enable it**.
* `glibc.cpu.hwcaps=SHSTK` enables it for a fully marked program and locks `shstk` and `wrss`: three system
  calls, enable + status + lock with mask ~0.
* An unmarked program, or an unmarked library added with `LD_PRELOAD`, stays off. The decision is the AND over
  all initially loaded objects. `hwcaps=SHSTK` with `x86_shstk=off` stays off.
* `dlopen` of an unmarked library with the shadow stack on:
  * default (`hwcaps=SHSTK`): refused with "rebuild shared object with SHSTK support enabled"; the process
    keeps its shadow stack.
  * `x86_shstk=on`: allowed, and the shadow stack **stays on**.
  * `x86_shstk=permissive`: allowed, and the shadow stack is turned off. In permissive mode nothing is locked
    beforehand. A multi-threaded process in this mode was not probed.
* Ubuntu ships the reference with the shadow stack opt-in. These probes cover only that build; whether another
  build defaults otherwise is not known from a black box.

## 3. What in libniji must change (with sizes)

R = needed for correctness whenever the shadow stack is on; M = needed only for some policy modes.
The negative control is the static bench linked with today's `setjmp.S`, `vfork.S` and `context.S`.
With the shadow stack on, longjmp, longjmp out of 10 frames, swapcontext and vfork all died with SIGSEGV (rc
139). The prototype's versions passed. Thread exit, cancellation and signals passed unchanged.

| item | what | size | R/M | evidence |
|---|---|---|---|---|
| A. Mark libniji itself | Today `ld-linux-x86-64.so.2`, `libc.so.6` and `libm.so.6` carry **no** x86 feature property. In `libc.a` only 28 of 365 members have one, so every static libniji program is unmarked: the AND of its inputs drops the property that D-X.144's start files carry. Under any "all objects marked" rule no libniji process would ever qualify. Fix: link the shared objects with `-z shstk` (after the audit in this table) and give every `libc.a` member the note (rustc's flag if the pinned stable toolchain has one; otherwise `cargo xtask so` adds `.note.gnu.property` per member), plus a `check_start_file`-style product check. | xtask ~60 lines + 1 product test | R (for any mode but off) | readelf of the products (build job `shstk-scope-so`) |
| B. The start-up decision and enable | Dynamic: AND the `GNU_PROPERTY_X86_FEATURE_1_AND` bits of the program, every `DT_NEEDED` object, `LD_PRELOAD`, ld.so and libc (the reader already parses the note, L14-12). Then enable from a frame that never returns: an asm `niji_machine_shstk_enter(fn, arg)` that calls `arch_prctl` and then calls a never-returning continuation (running the initializers, then `niji_machine_ld_enter`), plus `ARCH_SHSTK_LOCK` unless the mode needs to disable later. Static: the same decision for the one image in the Rust start (`niji_rt::start`), through the same asm helper. New rare logic goes cold (D-X start-up rules). | Rust ~50 lines, asm ~15, 1-2 syscalls | R | prototype (enable at entry, 1 syscall) |
| C. dlopen of an unmarked object after enabling | One check per newly mapped object, before its initializers run. Refuse: an error path, ~20 lines. Disable: per thread only (section 2), so a process-wide disable has to signal every thread (like the set*id broadcast), ~80 lines, and the process cannot be locked. Allow-list: ~30 lines plus a file format. | 20-80 lines | M | reference behaviour, section 2 |
| D. setjmp / sigsetjmp / longjmp / siglongjmp | setjmp records SSP (`rdssp`, +8) in a private slot of `jmp_buf` (offset 80, inside `__saved_mask` past the kernel's 8 bytes; only libc reads it). longjmp pops to it with an `incsspq` loop (255 per step). | asm +6 / +13 instructions (`setjmp.S`) | R | +9 user instr per pair, measured; control crashed |
| E. getcontext / setcontext / swapcontext / makecontext | swap and set switch shadow stacks with `rstorssp` + `saveprevssp` and record the token in `uc.__ssp[0]` (ABI offset 936) with a flag in `__ssp[1]`. The resume becomes `jmp *%r11` instead of `push; ret`. A same-stack setcontext pops with `incsspq` like longjmp. makecontext calls `map_shadow_stack` (measured 20.9 k kernel instructions, 1 fault, about 6 µs, once per context). Its trampoline must reach the function through `call`, so that the function's `ret` into `niji_machine_start_context` matches; the prototype covered only a function that never returns. Open: **nothing frees a makecontext shadow stack** (the API has no destroy). Options: reuse per `uc_stack.ss_sp` (coroutine libraries reuse stacks), free at thread exit, or accept the leak. | asm ~35 lines, Rust ~60 | R | swap: +12 user instr per switch, measured; control crashed |
| F. Thread creation | The kernel allocates the shadow stack. Pass the thread's stack size so the shadow stack is not RLIMIT_STACK-sized: create threads with `clone3` (`stack_size`; `niji_machine_clone3_run` exists) instead of `clone`. Without it every thread reserves 8 MiB, or 4 GiB under an unlimited rlimit. | asm/Rust ~30 lines | R (resource), M (size) | smaps, section 1.2 |
| G. vfork | The child's calls overwrite the shared shadow-stack slot of vfork's return. Both returns skip it (`incsspq 1`) and jump. | asm +7 instructions | R | measured; control crashed |
| H. posix_spawn's `clone3_run` (`CLONE_VM|CLONE_VFORK`) | The child calls on the parent's shadow stack below its SSP and never returns. Expected safe, but **not tested**: add a test. | test only | R (test) | none yet |
| I. Thread exit, cancellation, forced unwind | Worked unchanged. The `-fexceptions` path goes through libgcc's unwinder, which adjusts SSP itself. The C-cleanup path also passed in the negative-control build (today's `setjmp.S`), so it does not longjmp across frames. | none | - | measured, dynamic and static |
| J. Signals, sigaltstack, restorer | Worked unchanged: the kernel handles them. | none | - | measured |
| K. Tunables | `config.rs` knows `glibc.cpu.x86_shstk` (string) and `glibc.cpu.hwcaps`. To match the reference: `hwcaps=SHSTK` opts in, `x86_shstk=off` vetoes, `=on` keeps the shadow stack on through an unmarked dlopen, `=permissive` disables instead of refusing. | Rust ~40 lines | M | reference probes |
| L. The rest of the assembly | Checked every `rsp` load, `push`+`ret` and indirect jump in `csys/src/machine/*.S` and `csys/start/*.S` (Rust has no inline assembly). Only items D, E and G break the call/ret pairing. `mcount.S`, `tlsdesc.S`, `dl.S`, `call.S`, `vdso.S` and `entry.S`'s `ld_enter` (a `jmp`, no return) are balanced. | none | - | static scan |
| M. Tests and plants | One test per R item, run with the shadow stack on where the CPU has it, skipping with a printed reason elsewhere. Only the N100 class among the lab hosts has it, so CI coverage needs a runner there. | ~10 tests | R | - |

Total: about 450-550 lines across xtask, the loader, the start code and 3 assembly files, plus tests. That is
one medium story for R (A, B, D, E, F, G, H, M) and a small one per mode (C, K).

## 4. Compatibility risk

* **What is marked.** Ubuntu 24.04 on the Zen 1 build host (`bench/marking.sh` over `/usr/bin`, `/usr/sbin`,
  `/usr/lib/x86_64-linux-gnu` and its subdirectories, python lib-dynload and `/usr/libexec`): 2,238 marked, 68
  without any property note. The unmarked include `python3.12` and `libpython3.12`, `libLLVM`, `libgmp`,
  `libjpeg`, `libvpx`, `libaom`, `libmpg123`, `libsodium`, `libnspr4`, `libfreeblpriv3`, `libunwind`,
  `librsvg`, several gstreamer plugins, `gdb`, `busybox`, `btrfs-progs` and all `qemu-*` user emulators. These
  are mostly assembly-heavy codecs and crypto, JIT and emulator runtimes, and stack walkers.
* **Yohane's packages** were not reachable from this study (Yohane reports through GitHub issues). The same
  script, `handoff/SHSTK-SCOPE/bench/marking.sh`, can be run on a Yohane root. Note that Debian (unlike
  Ubuntu) does not necessarily build with `-fcf-protection`. Not checked here.
* **What breaks with "on + refuse".** Any marked program that later `dlopen`s an unmarked object: image
  loaders (libjpeg through gdk-pixbuf), media codecs (gstreamer, libvpx, libaom), Mesa's llvmpipe (libLLVM),
  NSS crypto (libfreeblpriv3) and Python extension modules built without the flag. They fail with a load error
  rather than a crash. Under "on + permissive" they run, but the whole process loses its shadow stack.
* **What breaks with "on" for code that is marked wrongly.** A JIT or coroutine library that switches stacks
  by hand (green threads, `ucontext`-free coroutines in asm, some GC runtimes) and was nevertheless built
  marked would crash with #CP. The hand-written assembly is usually the reason a package is unmarked, so the
  marking protects most of these cases.
* **Go and other runtimes.** Static Go binaries do not use libc and are unaffected. cgo binaries link Go's
  unmarked runtime objects, are unmarked, and so stay off. Java and Node JITs run in unmarked or marked builds
  depending on the distribution; a marked JIT has to manage SSP itself.
* **Resources.** 8 MiB of address space per thread (4 GiB under an unlimited stack rlimit until item F), one
  resident page per thread and per process, and +13 k kernel instructions per thread creation (+20 % wall time
  on the N100).

## 5. Policy options for the owner

| option | start-up cost | per-thread cost | risk | difference from the reference (this Ubuntu build) | needs |
|---|---|---|---|---|---|
| 1. Off (today) | 0 | 0 | the marking does nothing; no protection | none: same default | nothing. D-21.3 stays |
| 2. Opt-in, matching the reference: `glibc.cpu.hwcaps=SHSTK` enables for all-marked processes; refuse, on or permissive through `glibc.cpu.x86_shstk` | 0 unless asked; then +1-2 syscalls, +1 fault, +8-33 k kernel instr | +13.3 k kernel instr, +1 fault, 4 KiB RSS | low: only users who ask | none (same tunables, same default) | items A, B, C (refuse + permissive), D-H, K, M |
| 3. On by default for fully marked processes, dlopen of unmarked refused, locked | same as above, for every marked process | same, every thread | medium: the dlopen failures listed in section 4 | **yes**: default on where the reference is off | as 2, plus a differences row and a compatibility option, `glibc.cpu.x86_shstk=off` or `glibc.cpu.hwcaps=-SHSTK` (the reference's own spelling) restoring "off" |
| 4. On by default, permissive fallback (disable when an unmarked object is loaded; not locked) | as 3 | as 3 | low breakage, weaker protection: the process can drop it (an attacker able to call `arch_prctl` can disable it, because nothing is locked); per-thread disable needs a broadcast | yes | as 3, plus item C's broadcast |
| 5. Opt-out only (on, locked, unmarked dlopen allowed: the reference's `x86_shstk=on`) | as 3 | as 3 | highest: unmarked code runs under the shadow stack and may crash | yes | as 3 |

The difference row and compatibility option (options 3-5) would read, neutrally: "libniji enables the user
shadow stack for processes whose objects are all marked SHSTK", and the option would restore the reference's
default off. Ids would be reserved when a ruling chooses an option (`ops reserve --story <story> COMPAT`).

**Recommendation.** Option 2 now: it costs nothing unless asked for, changes no behaviour, and does the
correctness work (A, B, D-H) that every other option needs as well. Then decide between 3 and 4 with data from
a Yohane system (its marking survey and the dlopen failures above). Item A (marking libniji's own objects) is
the precondition for any of it. Until then, the property on the start files promises nothing at run time.

## 6. Not verified / open

* The reason for libniji's +33 k kernel instructions in the three-library row (section 1.1 hypothesis).
* posix_spawn under the shadow stack (item H), makecontext with a returning function (item E), and a
  multi-threaded permissive disable. None was run.
* Zen 3 (AMD implements the same instructions separately): not measured. It needs the coordinator's ledger
  claim (`handoff/SHSTK-SCOPE.md` lists the job, the copy and the duration).
* The workload kits were not run (section 1.3 subset instead).
* The reference's default comes from one distribution's build.
