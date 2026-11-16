# SYS-GAI-EAI-AGAIN handoff (checkpoint 1: context budget)

State: branch task/sys-gai-eai-again, cut from origin/next; role keke ~/w/n4. Code commit 75b1e0eb4, then this
handoff, the behaviour record and the plants file.

Done:
* Probe `crates/core/tests/oracle/gai_down.c` + `gai_down_run.sh` (unprivileged `unshare -r -m -n`, fixture /etc,
  fresh /dev/shm). 17 scenarios: unreachable (v4, v6), refused, two servers down, timeout, SERVFAIL, truncated with TCP
  refused, empty/missing resolv.conf, missing /etc/hosts / nsswitch.conf, `[!UNAVAIL=return]`, server coming up mid-run.
  Reference results recorded in `spec/behaviour/gai-down.md` (job sys-gai-eai-again-probe1 on keke, rc 0; output
  /dev/shm/claude/sys-gai-eai-again/probe-1/glibc.txt there).
* Before the fix libniji differed in 48 hunks: EAI_FAIL / NO_RECOVERY, or EAI_SYSTEM+ENOENT without /etc/hosts
  (the issue); gethostbyaddr NO_RECOVERY; getnameinfo numeric / EAI_NONAME; gethostbyname errno 0 on TRY_AGAIN;
  and the cache served answers found while DNS was down after it came back (recover_unavail).
* Fix (75b1e0eb4): new `LookupError::Unreachable` (core names/nss.rs): `unavail` for the rules, TRY_AGAIN/EAI_AGAIN for
  callers, not replaced by an earlier file error in `nss::run`. resolv_compat: host_miss maps no-server to it;
  host_by_addr_life no response -> TryAgain; both mark `Life` unknown on no response (cache keeps nothing, D-16.10).
  gai.rs: Unreachable -> EAI_AGAIN; getnameinfo TryAgain/Unreachable -> EAI_AGAIN whatever the flags.
  boundary netdb.rs: gethostbyname2 sets errno EAGAIN on TRY_AGAIN; _r returns EAGAIN.
* Test `crates/core/tests/gai_down_products.rs` (line-by-line differential; skips without userns/cc).

NOT VERIFIED yet: job sys-gai-eai-again-verify1 (keke; clippy, static, so, gai_down_products, resolv_compat unit
tests, probe diff) was still running at hand-back: `ssh keke '~/bin/jobwait sys-gai-eai-again-verify1'`, log
~/logs/sys-gai-eai-again-verify1.log.

Next step: read verify1; fix what it shows; sync; run the direct gate (`cargo xtask affected --run` with
--base origin/next) and plants (`handoff/SYS-GAI-EAI-AGAIN-plants.txt`, --timeout 2400, new /dev/shm dir); then the
gnulib test-getaddrinfo check (gnutls28 source built against the products, offline: must skip, not fail);
`dns_products` (needs sudo -n) should stay green: Unreachable reaches its `unavail` scenario only through the rules.
Open: networks (`net_by_*`) keep Unavailable for no server (not probed). Plan rows: D-16.1, D-16.2, D-16.10.
