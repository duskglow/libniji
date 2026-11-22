# LD-LOOKUP-COST: one symbol lookup at the reference's cost or better

Proposed by STARTUP-STRUCTURE-REVIEW (C2, rank 2). Kind: code (RETHINK of the lookup path). Size: M.

Premise: a symbolic relocation against libc costs 1,373 instructions on libniji against ~600 on glibc 2.39 (black
box: 200 imported functions minus none, proto1). Inside the empty main they cost 1,656 each (cg3). Per relocation
that is ~390 in reloc::target, ~640 walking hash chains through checked byte reads, ~250 rebuilding names and
versions, and ~150 in symbols::lookup. Each object ahead of the definer adds ~120, because nothing rejects cheaply
before the chain walk.

Change (design it; two approaches estimated before building, §11):
* validate each object's hash table, chains, symbol table and version tables once, at `Object::new`, then index
  inside the validated ranges;
* a GNU-hash bloom reject per object before the chain walk;
* versions by index through a per-object table built once;
* decode the relocation and its symbol once.
Loaded data is untrusted: a check moves to the one validation, none is dropped.

Must prove: per-lookup marginal <= 600 (200-import probe, both CPU classes), stretch 300; `/bin/true` -45k after
LIBC-SELF-BIND; no lookup-result change (the dynamic-link conformance suites; plants on the reject and version
paths).
