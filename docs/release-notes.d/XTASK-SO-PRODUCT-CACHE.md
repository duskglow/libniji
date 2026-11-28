---
section: changes
---
- **`cargo xtask so` skips work whose inputs are unchanged:** a build whose tree, staged headers, tools, xtask binary,
  environment, profile and layout match an earlier full build restores that build's products (each checked by sha256)
  instead of making them again, and a trace-at-build of an unchanged library archive reuses its lists.
  `NIJI_SO_CACHE=0` builds everything; the CI train (`NIJI_TRAIN`) never uses the cache. `cargo xtask bench` prints
  `step: bench/build`, `step: bench/counters` and `step: bench/rounds` lines.
