# LIMITS-CONFIG-SMALL: hardcoded-limits audit rows HL-16..HL-19

Branch `task/limits-config-small` (from origin/next e3b0caf60). Role keke:~/w/r2.

## State
- **HL-18 done** (`crates/boundary/src/report.rs`). `error_one_per_line`'s remembered file name has
  no limit but memory. The existing 4096-byte table holds the common case. Bytes past it go to a
  `niji_rt::chunks::Chunks` (`LAST_MORE`, backed by `KernelPages`), mapped when a longer name first
  needs it and never moved. With no memory, the name is not remembered and the message prints again.
  No new `unsafe`. Test: `conformance/c/s02_report.c` `c3_long_name` (clause s2_3_c_3), with an 8 KiB
  name: a repeat at another address is suppressed, and a name that differs only in its last byte is not.
- **HL-19 done** (`crates/boundary/src/users.rs`; D-17.10 amends D-17.8/D-17.9). `getpass` has no
  default bound. `niji_getpass_set_limit` stays as an opt-in bound. `NIJI_GETPASS_DEFAULT_LIMIT` is 0,
  which means no bound. The call returns 0 when no bound was in force, so save and restore round-trips.
  A line that memory cannot hold is drained, zeroed and fails with `ENOMEM`, like the over-bound
  `EOVERFLOW` path. Before, the stream path returned early and left the rest of the line unread. The
  differences row is removed: libniji now matches. Test: `sym_getpass_bound` (a 4 MiB line whole on
  stdin; `ENOMEM` under `RLIMIT_AS` and the next line read normally; the set bound on a pipe and a tty).
  The 4 MiB line goes through stdin, not a pty: a terminal in canonical mode passes at most 4095 bytes
  of a line (Linux N_TTY), so a pty cannot carry it. The fd (tty) path's `ENOMEM` drain mirrors the
  stream path but has no test of its own, for the same reason.
- **HL-16, HL-17 NOT done**. The coordinator accepted the challenge (#7807); a follow-up story is owed.

## HL-16 / HL-17 design (for the follow-up; estimate M)
Why it is not S: `sort::Policy`/`Table` (gai.conf) and `nss::Spec`/`Config` (nsswitch) are `Copy`
value types. Their process-wide copies live only as fixed-size encodings in atomic byte buffers
(`legacy::result_slots::Buf<N>`: `GaiConf.policy`, `Switch.lines` and `Switch.overrides`). The
encodings also hold a `u8` count. `legacy` and `rt` have no `unsafe` and no `UnsafeCell`, and no
`Pages` source (`Pages::words` is implemented in boundary), so they cannot keep a growable heap object
in a static.
1. Storage: keep each `Buf<N>` for the common case (16 sources per line, 32 rows per table) and add a
   `Chunks` overflow for the words past it, plus a `u32` length. Readers already hold the `Watched`
   lock or the override lock (`GaiConf::policy` reads `reload` outside the lock: move that flag to its
   own `AtomicBool`), so in-place rewrites under the lock are safe, and the chunks only grow.
   `Pages` must come from boundary: give `Switch::config`/`config_as_read`/`configure` and
   `GaiConf::policy` a `&P: Pages` parameter. There are 5 callers: `nss.rs` `Ctx::with_paths`,
   `nscache.rs` x2, `gai.rs` `sort_addrs`, `boundary/netdb.rs`. Or add a `pages` field to `Ctx`.
2. Values: `Spec { inline: [Step; 16], len, more: TryVec<Step> }` and `Table { inline: [Entry; 32],
   n, more: TryVec<Entry> }`. Both stay const-constructible (`TryVec::new` is const). `Config::EMPTY`
   needs `[const { None }; DATABASES]`. They lose `Copy`, so: `Config::spec`/`resolve` return `&Spec`
   (defaults from `static` items), `Ctx` loses `Copy` (19 copy sites), and there are about 47
   `spec()` sites, most of which compile unchanged through deref coercion. An allocation failure while
   parsing: the nsswitch line fails to parse (the defaults stand, and `__nss_configure_lookup` returns
   `ENOMEM` rather than `EINVAL`); a gai.conf table keeps the rows that fit, which needs a ruling, or
   falls back to the defaults.
3. Also seen and not in the audit's rows: `core/src/names/nss.rs` `rules()` normalises a `[...]` group
   into `[0u8; 256]`, so a longer action group does not parse. Fold it into the follow-up.

## Not to redo
- The reference probe: `conformance/c/s02_report.c` built against the host C library passes s2_3_c_3,
  so the reference also remembers an 8 KiB name (job limits-config-small-refprobe). getpass: the
  removed differences row recorded that the reference returned a 3 MiB line whole.

## Next step
Follow-up story for HL-16/HL-17 per the design above. Plan rows: plan/hardcoded-limits.md HL-16..HL-19;
D-17.8, D-17.9, D-17.10.

Full report: handoff/LIMITS-CONFIG-SMALL-report.md.
