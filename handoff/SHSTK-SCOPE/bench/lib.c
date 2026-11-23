/* SHSTK-SCOPE: a small shared library; built as lib1..3 (marked) and libunm (unmarked). */
#ifndef NAME
#define NAME lib_fn
#endif
int NAME(int x) { return x + 1; }
