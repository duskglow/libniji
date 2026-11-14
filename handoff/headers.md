# headers (SL-HEADERS-PLAN), handoff

**State:** branch task/sl-headers-plan. This is a planning story: it changed no code and ran no jobs.

**Deliverables**
* `plan/headers.md`: inventory, generation, proof, slices and rulings.
* `plan/headers-inventory.tsv`: 478 header names, each with its section and slice.
* `handoff/SL-HEADERS-PLAN/SL-HEADERS-1..20.md`: the slice briefs.
* `records/pending/SL-HEADERS-PLAN-1.toml`: the pending decision, series D-X.

**Inventory**
* Source: Devuan excalibur libc6-dev 2.41-12+deb13u4. The chain (gpgv `InRelease` → `Packages.xz` →
  `.deb`) was verified with the pinned keyring on keke; its SHA-256 matches `~/state/mirrors/`.
* Names only, from `dpkg-deb -c`. The reference (keke, 2.39) was listed with `dpkg -L`.
* There are 243 public headers, the same set in 2.39 and 2.41.
* No header name is in both libc6-dev and linux-libc-dev.
* Only kernel UAPI text was read: `linux/libc-compat.h`.

**Rulings open:** R-H0 to R-H7 (plan/headers.md §5). R-H0 blocks slices 2–4.

**Next step:** the owner rules on R-H0 to R-H3. Slice 1 can start now, and slice 2 after R-H0.

**Not verified:** no probe was run. The `__isoc*` and `_chk` mappings are inferred from the exports.
The reference compiler is GCC 13.3, while excalibur uses GCC 14.
