# GAI-ADDRCONFIG-LOOPBACK handoff

State: branch `task/gai-addrconfig-loopback`, code head 7d75c2a7c (on origin/next 9cfa6166c). Role shiki:~/w/sh1.
Decision in force: D-16.2 (revised, AI_ADDRCONFIG clause at the end of the row); owner ruling #8731 "match stock".

## Done
- Probe (black box, stock 2.39 vs libniji static, `unshare -r -n -m` namespaces with fixture /etc:
  hosts files only). Probe sources and runners: ~/work/gai-addrconfig-loopback/ on shiki (gai_probe.c,
  probe_run.sh = 16 scenarios, probe_run2.sh = 5 more). Outputs out_<scenario>_{host,niji}.txt there.
- Rule observed on the reference:
  - IPv4 configured iff some interface (up or down, any name incl. `lo`) has an IPv4 address other than
    exactly 127.0.0.1 (127.5.0.1 on lo or d0 counts; 169.254/16 counts; 127.0.0.1 on d0 alone does not).
  - IPv6 configured iff some interface has an IPv6 address other than exactly ::1 (link-local counts,
    global on lo counts).
  - Narrowing before the node is read: both configured -> unchanged; one -> AF_UNSPEC becomes it, the
    other family is EAI_NONAME; neither -> AF_UNSPEC unchanged, AF_INET/AF_INET6 EAI_NONAME. The
    EAI_NONAME comes ahead of EAI_ADDRFAMILY, EAI_SERVICE, EAI_SOCKTYPE. INET6|V4MAPPED keeps mapped
    answers whenever IPv6 is configured.
- Implemented in crates/legacy/src/gai.rs (`configured`, `addrconfig`, pub `addrconfig_families`); old
  post-filter removed. Docs: differences row removed; D-16.2, spec/behaviour/netdb.md, spec line
  (§16.2 s16_2_c_2_5, clauses.tsv regenerated). Tests: s16_2_c_2_5 (host case + full probed table),
  netdb_diff.c gains 7 loopback-only rows (end-to-end vs reference under sudo harness).
- clippy (workspace, all targets, all features, -D warnings) and `cargo xtask static` green on the role
  (job gai-addrconfig-loopback-lint2, same tree as 7d75c2a7c).

## Jobs started (shiki), on the 7d75c2a7c tree
- gai-addrconfig-loopback-gate1 (direct-tier gate, --base origin/next)
- gai-addrconfig-loopback-plants1 (plants file ~/work/gai-addrconfig-loopback/plants.txt: 4 plants;
  gac-callsite relies on netdb_products, which skips without passwordless sudo: if it SURVIVES for that
  reason, look at its log and report it, do not loosen it)

## Exact next step
1. `~/bin/jobwait gai-addrconfig-loopback-gate1 gai-addrconfig-loopback-plants1` on shiki; fix reds.
2. Final probe on the new products: `cargo xtask so` on the role (jobrun), then rerun probe_run.sh and
   probe_run2.sh as jobs (probe3/probe4); every C (AI_ADDRCONFIG) row must be SAME. Known non-AI_ADDRCONFIG
   diffs, not this story: `ip6-localhost` AF_INET (stock 127.0.0.1, libniji EAI_NONAME; the gethostent
   `::1` row, D-16.2) and the passive AF_UNSPEC wildcard order when NO interface address exists at all
   (stock `:: 0.0.0.0`, libniji `0.0.0.0 ::`; lo down). Report both INFORMATIONAL.
3. Write handoff/reports/GAI-ADDRCONFIG-LOOPBACK.json, sync, precheck --jobs <labels> --head <sha>,
   handback-check, hand back.

Probe-only rows (no test on the gate hosts): the one-family branch's wiring (h.family rewrite) and the
exact-127.0.0.1 IPv4 rule; the pure table test covers the decision itself.
