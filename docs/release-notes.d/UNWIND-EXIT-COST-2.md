---
section: changes
---
- **`pthread_exit` and cancellation unwind the stack with fewer instructions:** in a dynamic program a plain-C
  `pthread_exit` at call depth 0 takes 32,074 user instructions instead of 59,558, at depth 10 54,290 instead of
  101,326, and `pthread_cancel` of a thread 3 frames deep 39,627 instead of 72,860 (stock's `pthread_exit` at depth 0:
  8,802, so the cost is still higher than stock's).
