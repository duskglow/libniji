# LD-VERSION-INDEX: version names by index per object, replacing the hashed version memo

Kind: code. Priority: next after LD-LOOKUP-RETHINK lands. Size: M. Model: opus.
Premise (LD-LOOKUP-RETHINK, handoff/LD-LOOKUP-RETHINK.md): on the C library's names a lookup pays 271 instructions
in version work against 125 with one version; the difference is misses of the 64-slot memo, which is direct-mapped
by (index, side, string-table address): memo_fill is 90 per lookup inclusive on 200 libc imports, 4.2k on an empty
main and 7.3k on `/bin/true` (release, Haswell-EP, ASLR off). Slots follow where the tables are mapped, so the miss
count can change with ASLR (measure it: run the kit with ASLR on, 101 starts, report the spread).

## The change
Give each object, once at load (`Object::build`, where `versym_with_versions` already walks the tables), an array
indexed by version index (definitions and requirements share the index space) holding the name's offset and length
and the ELF hash (`vd_hash` / `vna_hash`), so both sides answer by index: no memo, no walk, no string compare
unless hashes are equal. The `Object` view must not grow (0x128 assertion): keep the arrays beside the views
(per-object slices on the start-up stack or the loader's arena), validated once (index bounds, name inside the
string table, NUL-terminated). A check on loaded data moves to load time, never drops.

## Proof
The kit's libc / hit-ver / hit-1st per-lookup rows and empty main, three libraries, `/bin/true` before and after
(instructions; faults), ASLR off and on; tests that every (reference, definition) version pairing answers as the
walks (`version_memo_answers_as_the_walks` extended); plants; the direct gate; the start-up checks.
Estimate: -150..-200 per lookup on libc names; empty main -3..-5k, `/bin/true` -6..-8k.
