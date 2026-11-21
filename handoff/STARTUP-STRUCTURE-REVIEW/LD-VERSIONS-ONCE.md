# LD-VERSIONS-ONCE: version requirements checked by hash, once

Proposed by STARTUP-STRUCTURE-REVIEW (C5, rank 6). Kind: code. Size: S.

Premise: `check_versions` costs 20,041 on the empty main and 43,311 on `/bin/true` (cg3). It walks version
definitions and compares names, with `Verdefs::next`, `def_name_from` and memmove in the profile.

Change: compare the stored ELF hash first (vd_hash / vna_hash), name only on a hash match; one verdef index per
object (shared with LD-OBJECT-ONCE).

Must prove: `check_versions` <= 5k (empty main), <= 10k (`/bin/true`); version-error conformance unchanged.
