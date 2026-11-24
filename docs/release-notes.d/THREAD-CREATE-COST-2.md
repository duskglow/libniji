---
section: changes
---
- **Creating and joining a thread costs far fewer instructions:** a thread that returns, then is joined, takes about
  1,200 user instructions in a static program and 1,350 in a dynamic one, against about 36,000 before (stock glibc:
  2,285); eight threads alive at once cost about 1,070 to 1,200 each (stock: 2,441). A thread that calls
  `pthread_exit` costs 1,292 in a static program and 31,034 in a dynamic one (before: 36,109 and 65,798; stock: 9,558).
  The dynamic `pthread_exit` rows are still above stock's, because the unwinder walks the thread's stack.
