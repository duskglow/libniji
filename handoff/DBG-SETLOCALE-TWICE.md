# DBG-SETLOCALE-TWICE: one location for `break setlocale` (and ten more exports)

State: branch task/dbg-setlocale-twice, pure renames of internal functions that shared an exported C name.

Premise (the brief stated none): gdb resolves `break NAME` to every debug-info function called NAME, so an internal
function sharing an export's name gives extra locations; the fix is in the names, `dbg-system.sh` stays strict.

## Renames
| was (internal) | now |
| --- | --- |
| boundary `locale::setlocale` | `locale_set_name` |
| legacy `global_locale::setlocale` | `global_setlocale_request` |
| legacy `cstring::argz_delete / argz_insert / envz_add` | `argz_delete_list / argz_insert_list / envz_add_list` |
| legacy `time_compat::asctime` | `asctime_text` |
| legacy `sys_compat::confstr`, `sysconf_table::confstr` | `confstr_into`, `confstr_value` |
| legacy `num_compat::ecvt / fcvt` | `ecvt_digits / fcvt_digits` |
| legacy `mb_compat::wctob` | `wctob_in` |
| legacy `ctype_tables::wctype`, core `ctype::wctype` | `wctype_in`, `wctype_class` |
| boundary `ret::time`, core `Tzif::time` (method) | `time_result`, `time_value` |

## Scan (system-flavour libc6-dbg built from the head, on mei)
gdb `info functions -n` on libc.so.6 with its debug file, names intersected with `nm -D` exports. Before: 11 names with
two or more definitions (the ten above, plus `read`). After the renames: only `read`. `break NAME` for the ten and for
`setlocale` is now one location each (`Breakpoint 1 at ...`, not `<MULTIPLE>`).

## Left over (INFORMATIONAL)
`break read` still has 28 locations: libc's own `read` plus inlined `core::ptr::read`, atomic `read`, and methods named
`read` (core/src/stream.rs, bytes.rs, legacy/src/intl.rs, printf_va.rs, boundary/patterns.rs, aio.rs, rt/unwind.rs).
Most are the Rust standard library's, which cannot be renamed. No dbg check uses `read`. Other exported names that are
only methods/inlined std functions would show up the same way; a guard needs a built libc6-dbg (about 15 min), so it
is not a unit test.

## Proof
dbg-system.sh (strict `^Breakpoint 1, `) on freshly built packages: trixie pass=9 fail=0, excalibur pass=9 fail=0.
Plants: none (pure renames; a plant would only mutate a name the compiler already checks).

## Next step
A static or train check that lists exported names defined twice in a built libc's debug info (the scan in this note),
with `read` handled (allow-list of std/inlined names).
