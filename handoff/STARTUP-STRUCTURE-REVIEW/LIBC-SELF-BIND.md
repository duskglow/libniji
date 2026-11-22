# LIBC-SELF-BIND: libc.so.6 binds its own function references at link time

Proposed by STARTUP-STRUCTURE-REVIEW (plan/reviews/STARTUP-STRUCTURE-REVIEW.md, C1, rank 1). Kind: code. Size: M.
Needs a ruling first (ABI: which libc-internal calls a program's own definition reaches).

Premise: 277 of the 285 names behind libc.so.6's 288 symbolic relocations are defined in libc.so.6 itself. Each start
resolves them through the whole scope (~1,650 instructions each, ~460k per start). A relocation-rewrite prototype
(proto1, selfbind.py "keep") bound them with the malloc family left preemptible: empty main 742,449 -> 328,846,
3 libraries 985,293 -> 482,090 (Zen 1, kit form). The interposition probe was unchanged: strdup reaches the
program's malloc, fopen never reaches the program's open, as on the reference.

Change: link libc.so.6 so that function references to its own definitions bind locally (GNU ld
`-Bsymbolic-functions` semantics) except a dynamic list that stays preemptible. That list holds the malloc family
(manual, "Replacing malloc") and every family where a reference probe shows that its internal callers reach a
program's definition. Exported data stays symbolic, so COPY relocations keep working: stdin, stdout, stderr, optarg,
optind, opterr, optopt, program_invocation_name, program_invocation_short_name, the _IO_2_1_* objects,
re_syntax_options, the obstack handlers. The prototype bound those too; a shippable link must not.

Must prove: empty main <= 400k user instructions (from 752k in the root form), 3 libraries <= 560k, both CPU
classes. Probes (conformance, both libraries):
* a program defining the malloc family still serves strdup, fopen-free paths and every other internal allocation
  the reference routes to it;
* a program with COPY relocations of stdout and optind sees getopt and stdio use its copies;
* one probe per bound family (pthread, aio, sem, mq, timer/clock, C11 threads, dl*, ns*, the rest) records whether
  the reference's internal callers reach a program definition; each family where they do stays preemptible.
Differences row only if a family has to differ (then a ruling).

Read first: the report §5 (interposition) and §6; D-27.11, D-27.27; xtask so's libc link.
