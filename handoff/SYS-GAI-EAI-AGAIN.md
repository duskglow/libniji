# SYS-GAI-EAI-AGAIN handoff (SYS-GAI-EAI-AGAIN-2: finished)

State: branch task/sys-gai-eai-again-2 (continues task/sys-gai-eai-again), cut from origin/next; role keke ~/w/n4.
Code commits 75b1e0eb4 (the mapping), 5af19faad (gethostbyname_r errno, probe line, plants), 60afc0576
(s16_4_c_0 expected the old Unavailable: found by the run-2 gate), then the recover_dnsfirst scenario. Report:
handoff/reports/SYS-GAI-EAI-AGAIN-2.json.

Done:
* Probe `crates/core/tests/oracle/gai_down.c` + `gai_down_run.sh` (unprivileged `unshare -r -m -n`, fixture /etc,
  fresh /dev/shm), 17 scenarios; reference results in `spec/behaviour/gai-down.md`. Differential test
  `crates/core/tests/gai_down_products.rs` (line by line; skips with a note without userns/cc).
* Fix: `LookupError::Unreachable` (core names/nss.rs): `unavail` for the rules, TRY_AGAIN / EAI_AGAIN for callers.
  resolv_compat: host_miss maps no-server to it; the address lookup without a response is TryAgain; both mark the
  cache `Life` unknown on no response (D-16.10: nothing kept). gai.rs: Unreachable -> EAI_AGAIN; getnameinfo
  TryAgain/Unreachable -> EAI_AGAIN whatever the flags. boundary netdb.rs: gethostbyname2 and (run 2)
  gethostbyname2_r set errno EAGAIN on TRY_AGAIN, as the reference does (probed: `_r` returns EAGAIN, `*h_errnop` 2,
  errno EAGAIN; recorded in the behaviour table). Scenario recover_dnsfirst (hosts: dns files, 192.0.2.10 in
  /etc/hosts) makes the address lookup's cache rule visible: plants-1 showed addr-cache-keeps SURVIVED without it. The probe prints `_r`'s h_errno/errno only on failure (on success
  gethostbyname(3) gives them no meaning).
* Issue #8 offline reproduction (own small program, host /etc, `unshare -r -n`, lo down and lo up): reference and
  libniji both print `www.gnu.org: -3 Temporary failure in name resolution`, `localhost: 0 ok` (job
  sys-gai-eai-again-2-repro on keke). gnulib's test-getaddrinfo itself was not built: no gnutls28 source on the build
  host; the coordinator's note allows the brief's reproduction instead. It skips on EAI_AGAIN, which this shows.
* Plants: `handoff/SYS-GAI-EAI-AGAIN-plants.txt`, one per site of the fix (11, all CAUGHT in plants2) plus one equivalent (`nss::found`'s
  arm: called only with passwd answers).

Jobs on the final head (keke): sys-gai-eai-again-2-gate2 (direct tier), sys-gai-eai-again-2-plants2,
sys-gai-eai-again-2-repro (reproduction; product code unchanged since). Superseded: -gate (s16_4_c_0), -plants
(addr-cache-keeps survived: fixed by the new scenario). Results: gate2 6051 passed / 0 failed; plants2 11 CAUGHT, 1 EQUIVALENT, 0 SURVIVED; repro2 fail=0.

Open (out of scope, #6124): the networks database's `dns` source (getnetbyname, getnetbyaddr and `_r`) keeps
`Unavailable` when no server answers; not probed. Probe to run: gai_down.c's fixtures with `networks: dns` in
nsswitch.conf (unreach and refused scenarios), print getnetbyname("x.test") / getnetbyaddr(0xc0000200, AF_INET)
result, h_errno, errno and getnetbyname_r's return, `*h_errnop`, errno; compare with libniji.

Not to redo: the probe matrix; the mapping. `dns_products` (needs sudo -n) reaches `unavail` only through the rules.
Plan rows: D-16.1, D-16.2, D-16.10.
Next step: none for this story; the networks probe above if the coordinator briefs it.
