# DOCS-CATCHUP-1126 handoff

State: branch task/docs-catchup-1126, docs only.

Done: a differences-from-glibc.md row for secure-mode `$ORIGIN` (D-27.16, rulings #7094, #7190); one sentence on the
later heap segment (D-3.24, HEAP-SECOND-NODE-MAP) in docs/architecture.md.

Already on head, so dropped: release-notes entries for #13, #6, the getaddrinfo_a correction and the DL-RUNPATH
`$ORIGIN` note (docs/RELEASE-NOTES.md, 11-21.7); install-packages.md and the locales Description describe the
charmaps. `libc-l10n` is still a documentation-only placeholder, correctly.

Open: the new row registers no COMPAT option (the reference's trusted-directory case is unobserved). Ruling
wanted: accept none, or reserve a COMPAT id.

Next step: land. Gate: `cargo xtask static` green on the role.
