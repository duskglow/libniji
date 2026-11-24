# LD-RELOC-FLAT: one flat symbolic-relocation path over a view validated at load

Kind: code. Priority: after LD-VERSION-INDEX. Size: L (split in two if needed: decode/store, then request/hash).
Model: opus, Effort: max.
Premise (LD-LOOKUP-RETHINK, handoff/LD-LOOKUP-RETHINK.md, phase table): of a 1,059-instruction hit (after D-27.91),
relocation decode and dispatch are 191, the target-address check and store 64, the referencing symbol, name and
request 192 and the name hash 76: 523, against the reference's whole lookup of 553. The cost is layers (closures in
`reloc::one`, `Result`/`Option` per read, per-read range checks through the image), not work: the reads themselves
are a handful of loads.

## The change
Validate once per object at load what the symbolic relocations read (relocation table bounds and entry size, symbol
count from the hash table, string table ending in NUL, `versym` length, each writable segment's range for the
relocation targets) and give the relocation pass one straight loop over the entries with plain slice indexing
inside those ranges: decode, symbol, name and hash in one pass, the scope walk of D-27.91, one store. A check on
loaded data moves to load time or folds into one range compare, never drops (owner 11-17); errors keep their kind.
Keep the hot loop byte-lean and new rare logic in one cold function (start-up rules, developer-flow §9).

## Proof
Per-phase split before and after (the kit's attr.py / phase2.py), the kit's per-lookup rows, empty main, three
libraries, `/bin/true` (instructions, faults); malformed-object tests for every check that moved; plants; the
direct gate; the start-up checks. Estimate: -320..-370 per lookup; empty main -19..-22k, `/bin/true` -33..-38k.
