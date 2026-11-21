# DL-NAME-PATH-2: every object's reported name is the path it was loaded from (rest of Yohane #24)

State: branch `task/dl-name-path-2`, cut from origin/next 02fd586b8. Code, tests and the spec fix are committed and
pushed; the head and job results are in handoff/reports/DL-NAME-PATH-2.json and the hand-back.

## What changed

- `crates/boundary/src/ld/link.rs`: the program's `$ORIGIN` at start comes from one cold function, `Origin::dir`.
  It is called only when a search can expand the token: `$` in the program's DT_RPATH, its DT_RUNPATH, LD_LIBRARY_PATH
  or the searched name. That is one `readlinkat` and no extra system call for programs without `$ORIGIN`; the reference
  also reads `/proc/self/exe` only then (strace).
  - Kernel-started: the directory of `readlink("/proc/self/exe")`. When that cannot be read the origin is unknown, so
    `LD_ORIGIN_PATH` stands in (the existing `expand` rule).
  - `ld.so PROGRAM`: the working directory + `/` + the path's directory as given (no `/` doubled when the cwd is `/`),
    or just the directory as given when getcwd fails.
  - The paths found under that origin are what the start-time link maps are named by. dladdr, dl_iterate_phdr,
    dlinfo(RTLD_DI_LINKMAP) and `_r_debug` all read those link maps, so all four now report the same full path.
  - `Origin` lost its `aux` field and `entry.rs::run` lost its `aux` parameter: `AT_EXECFN` is no longer read for
    `$ORIGIN`.
- `crates/ld/tests/object_names.rs`: 4 tests, every probed case (below). The plants are in
  handoff/DL-NAME-PATH-2-plants.txt: the bare DT_NEEDED name as `l_name`, no cwd join, and each of the three token gates.
- `spec/security/threat-model.md` rule 2 and its row in the summary table: they said the program's `$ORIGIN` follows
  `AT_EXECFN`, "not `/proc/self/exe`". The probes below contradict that, so both now describe `/proc/self/exe` with
  the `LD_ORIGIN_PATH` fallback.

## Probe-settled behaviour (glibc 2.39, black box; Zen 1 host; probes in the run's scratch, not in the tree)

| case | reference name (P = the probe's absolute directory) | libniji before | after |
|---|---|---|---|
| DT_RUNPATH / DT_RPATH `$ORIGIN/lib`, started `./bin/prog` | P/bin/lib/libA.so | ./bin/lib/libA.so | same as reference |
| started by absolute path | P/bin/lib/libA.so | same | same |
| started `./link/prog` (link -> bin) | P/bin/lib/libA.so (links resolved) | ./link/lib/libA.so | same as reference |
| started `./alias/m` (a link to the program file in another dir) | P/bin/lib/libA.so | libA not found | same as reference |
| `/proc` hidden (tmpfs over it, `unshare -r -m`) | origin unknown: load fails; with LD_ORIGIN_PATH, `<that>/lib/libA.so` as given | ./link/lib/libA.so | same as reference |
| cwd removed, absolute start | P/bin/lib/libA.so | same | same |
| `ld.so ./bin/prog`, `ld.so ../link/prog` | P/./bin/lib/libA.so, P/bin/../link/lib/libA.so (textual) | ./bin/lib/..., ../link/lib/... | same as reference |
| LD_LIBRARY_PATH `bin/lib`, `./bin/lib`, `bin/lib/../lib`, `link/lib` | kept as given (`bin/lib/libA.so` ...) | same | same |
| LD_LIBRARY_PATH `$ORIGIN/lib` | P/bin/lib/libA.so | ./bin/lib/... | same as reference |
| dlopen relative path (`./bin/sub/libB.so`, `bin/sub/../sub/libB.so`) or absolute path | kept as given | same | same |
| dlopen bare name via relative LD_LIBRARY_PATH | `bin/sub/libB.so` | same | same |
| ldd `./bin/prog` | `P/./bin/lib/libA.so` | same | same |
| dladdr of the program itself | argv[0] as given | same | same |

## Not in scope (found by the probes; route)

- **DL-RUNPATH cause** (bare-name `dlopen` through the program's DT_RUNPATH/DT_RPATH `$ORIGIN` fails, and so does
  `dlopen("$ORIGIN/sub/libB.so")` from the program). dlopen's scope for object 0 (`dlopen.rs::scope_of`,
  `dladdr.rs::origin`) takes `$ORIGIN` from the program's stored path, which is empty, so the program's origin is
  `None` and every `$ORIGIN` element is skipped. The fix is to publish the start's `Origin::dir` result, or call the
  same lookup lazily from dlopen. That is one place, shared with this story's function.
- `dlopen("libA.so", RTLD_NOLOAD)` for a start-time library with no DT_SONAME, named `libA.so` in the program's
  DT_NEEDED: the reference returns its handle, libniji returns NULL (it matches only the path or DT_SONAME). This is
  a loader story.
- `ld.so prog` (a program name without `/`): the reference treats it as a library name to search for and fails;
  libniji opens it from the cwd. Informational; not probed further.

## Decisions

- Pending `D-NEW` (no id reserved this run): "the program's `$ORIGIN` is the directory of `/proc/self/exe`, looked up
  lazily (token in DT_RPATH/DT_RUNPATH/LD_LIBRARY_PATH/name); unknown when unreadable; `ld.so PROGRAM`: cwd-joined
  path as given". It needs an id from `ops reserve --story DL-NAME-PATH-2 D-27`, then a row in plan/07-decisions.
- RULING NEEDED (trust boundary): secure mode now also takes `$ORIGIN` from `/proc/self/exe`, not `AT_EXECFN`. The
  trusted-directory check (rule 2) is unchanged. A kernel-resolved path cannot be chosen through a link name, and it
  matches the reference. Recommend accept.

## Layout

The change is in ld.so's start link only. It adds one cold function and a gate on the bare-name search path, and
libc.so.6's hot text is untouched. ld.so has no traced layout yet (§9, LAYOUT-LDSO-ORDER), so the layout-bins diff
against the base was not taken; the start-up checks job (`cargo xtask test` and the full-profile hot_text_products) is
in JOBS.

## Next step

Once landed, the coordinator can comment on duskglow/libniji#24 so Yohane can verify. The case Yohane hit is a
library found at start through the program's `$ORIGIN`, which was reported as `./libstart22.so`. Then: DL-RUNPATH
(cause above), and reserve the D id.
