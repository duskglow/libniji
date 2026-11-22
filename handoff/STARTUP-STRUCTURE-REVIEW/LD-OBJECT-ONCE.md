# LD-OBJECT-ONCE: one validating pass per loaded object

Proposed by STARTUP-STRUCTURE-REVIEW (C3, rank 4). Kind: code. Size: M. After LD-PHDR-ONCE.

Premise: each loaded object costs 93,819 instructions on libniji against 11,958 on the reference (3-library
marginal, cg3). Of that: `Object::new` ~30k for libc (91,584 for 3 objects), `Dynamic::once` linear rescans
(28,449; +7k per object), `dlopen::check` whole-file validation ~9k per object, path search ~12k per library, plus
~36k of other objects' lookups walking past it (LD-LOOKUP-COST).

Change: one pass per object that validates and records the dynamic tags (an indexed table), the GNU-hash parameters
and the version tables, with the whole-file check folded into it. Version-name tables are built only for the
versions referenced. Fold, never drop a check.

Must prove: per-object marginal <= 25k (3-library minus empty main, / 3); empty main -60k.
