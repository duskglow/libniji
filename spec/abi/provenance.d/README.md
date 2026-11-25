# Provenance of the spec/abi tables: one file per story

A story that regenerates or hand-edits a table in `spec/abi/` (a probe rerun, rows spliced in, a hand respelling)
records how it did so in a fragment in this directory. It does not add a paragraph to `spec/abi/PROVENANCE`.
`spec/abi/PROVENANCE` is generated (D-X.172). Its first part is the current counts of the tables (rows, types,
headers, names), counted from the tables. After that come the fragments in order. Every story writes its own
file and nobody edits a count by hand, so two header stories merge in either order.

## Adding a fragment

Create `spec/abi/provenance.d/<STORY-ID>.txt` (capitals, digits and `-`):

```text
---
order: 210
---
visibility.tsv (MY-STORY): header_probe.py --headers foo.h on the reference system (ldd ..., GCC 13.3): 12 rows
added, the committed rows of foo.h reproduced unchanged.
```

* `order` places the paragraph in the history: take the highest `order` in this directory plus 10. Two stories
  that pick the same number still merge cleanly and are ordered by file name.
* The body is one paragraph: no blank lines, no trailing white space, LF line ends.
* Give the counts of your own run (rows it added or changed, the size of a full run you spliced from). Leave out
  the totals of the tables: the generated part states those.
* `spec/tools/abi_extract.py` writes its own fragment for a full run and for `--empty-versions-only`
  (`--story <STORY-ID>`).

## Regenerating

You do not regenerate the view on your branch. `spec/abi/PROVENANCE` is train-owned, like the decision tables
(D-X.55, D-X.59):

* To read the view with your fragment and tables, run `cargo xtask records-gen`, then read
  `target/records-views/spec/abi/PROVENANCE`.
* The static check `records-gen` names the view when it is stale. Off the train that is advisory. On the train,
  which regenerates the view in place after each merge, it is fatal.
* A malformed fragment fails `records-gen` and `cargo xtask static` everywhere.
