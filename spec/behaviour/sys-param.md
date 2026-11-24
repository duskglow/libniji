# `<sys/param.h>` arithmetic macros (spec 8.12)

Clauses: s8_12_c_0 to s8_12_c_5. Sources: the man pages MAX(3), roundup(3) and powerof2(3) (Linux man-pages 6.7),
and black-box probes of the reference (glibc 2.39 as installed on Ubuntu 24.04, x86_64, gcc 13). Nothing here
describes or derives from the reference's header text: only what compiled programs observe.

Probe: `crates/core/tests/oracle/sys_param_diff.c`, built `gcc -std=gnu17 -O1` with the reference's headers and run
with an empty environment; output recorded in `sys-param.glibc.txt`. Each line is an item, its value and the type of
its result (`_Generic`), or `evals <a> <b>`: how many times each argument was evaluated (arguments are calls that
count). The tests (`crates/core/tests/s08_numeric_clauses.rs`, `s8_12_*`) build the same probe with libniji's
`libc-include/` and compare each macro family's lines with this record.

Settled by the probes:

* Visibility: all nine are defined in every configuration of `spec/abi/ftm-configs.tsv` the headers accept (strict
  C89 included), C and C++; with `-std=gnu17 -D_TIME_BITS=64` alone the header itself is rejected, so nothing shows.
* Constant expressions: `MIN`, `MAX`, `howmany`, `roundup` (constant steps 3 and 8) and `powerof2` of constants work
  in a C `enum`, `_Static_assert`, an array bound and a C++17 `constexpr`.
* A program's own definition made before the include: `howmany` is kept; `MIN`, `MAX`, `roundup`, `powerof2`,
  `setbit`, `clrbit`, `isset`, `isclr` take the header's meaning, without a diagnostic under `-Wall -Wextra -Werror`.
  A definition after the include draws the usual redefinition diagnostic.
* `MIN`/`MAX` choose the second operand on equal operands and with a NaN operand; the chosen operand is evaluated
  twice. `MIN (-1, 1u)` is `1u` (usual arithmetic conversions); a mixed-sign `MIN` is silent under `-Wall -Wextra`.
* `roundup` with a power-of-two step known at compile time clears low bits (`roundup (-5, 4)` is -4, `roundup (5, 0)`
  is 0); with a run-time step it divides (`roundup (-5, 4)` is 0) and evaluates the step three times.
* `setbit`/`clrbit` are expressions with the element's new value; the bit index counts in the array's own elements
  (`setbit (int_array, 13)` sets bit 5 of element 1); `isset` gives the masked bit (32 for bit 13), `isclr` 1 or 0.
