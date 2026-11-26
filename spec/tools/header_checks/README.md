# header_checks: the checks of one header slice, and their fragments

`spec/tools/header_check.py` loads every `<slice>.py` here in file-name order, and right after each one the
fragments `<slice>.d/*.py` in file-name order. A module or fragment may define `units(ctx)` ((configuration,
title, unit text) triples), `problems(ctx)` (FAIL lines), `p1_skip` and `must_fail` (header_check.py's docstring).
The units of one configuration run in load order, so a fragment's units run after its slice module's and after
the fragments that sort before it.

How a header story adds checks for a new family (HEADERS-FRAGMENTS):

* Add a new fragment `<slice>.d/<NN>-<family>.py`, `NN` a two-digit order. Take a number that no other fragment of the
  slice has. Do not edit the slice module or another story's fragment, so two stories never touch the same lines.
* Shared helpers come from `header_check` by name (`from header_check import expand`). The context `ctx` holds the
  tables (`ctx.vis`, `ctx.macros`, `ctx.layouts`, ...) and `ctx.exact(header, names)`.
* A directory `<slice>.d/` without `<slice>.py` beside it is an error, so a fragment is never skipped silently.
