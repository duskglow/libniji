/* sys/param.h's function-like macros (spec 8.12): what each evaluates to, the type of its result, how many times
   each argument is evaluated, and that the results usable in an integer constant expression are. A black-box probe:
   it includes only public headers and prints what it observes, one line per item, so the same program built with the
   reference's headers and with libniji's (libc-include/) can be compared line by line.
   Lines: `<item>\t<value>\t<type>` for a value, `<item>\tevals <a> <b>` for evaluation counts. */
#include <sys/param.h>
#include <stdio.h>
#include <limits.h>
#include <math.h>

#define TYPE(x) _Generic ((x), _Bool: "_Bool", char: "char", signed char: "signed char", unsigned char: "unsigned char", \
  short: "short", unsigned short: "unsigned short", int: "int", unsigned int: "unsigned int", long: "long",           \
  unsigned long: "unsigned long", long long: "long long", unsigned long long: "unsigned long long", float: "float",     \
  double: "double", long double: "long double", char *: "char *", const char *: "const char *", default: "other")

#define V(e) printf ("%s\t%llu\t%s\n", #e, (unsigned long long) (e), TYPE (e))
#define S(e) printf ("%s\t%lld\t%s\n", #e, (long long) (e), TYPE (e))
#define D(e) printf ("%s\t%a sign=%d\t%s\n", #e, (double) (e), signbit (e) != 0, TYPE (e))

/* Arguments that count their evaluations. The values come through volatile objects, so nothing folds them. */
static int na, nb;
static volatile long long va, vb;
static long long A (void) { na++; return va; }
static long long B (void) { nb++; return vb; }
static int Ai (void) { na++; return (int) va; }
static int Bi (void) { nb++; return (int) vb; }
static unsigned char bits[8];
static unsigned char *Arr (void) { na++; return bits; }
#define EVALS(what, a, b, e) do { va = (a); vb = (b); na = nb = 0; (void) (e); printf ("%s\tevals %d %d\n", what, na, nb); } while (0)
#define RV(what, a, b, e) do { va = (a); vb = (b); na = nb = 0; long long r_ = (long long) (e); printf ("%s\t%lld\t%s\tevals %d %d\n", what, r_, TYPE (e), na, nb); } while (0)

/* Integer constant expressions: each line compiles only if the macro's result is one. */
enum { C_MIN = MIN (3, 7), C_MAX = MAX (3, 7), C_HOWMANY = howmany (10, 3), C_ROUNDUP = roundup (10, 3),
       C_ROUNDUP2 = roundup (10, 8), C_POWEROF2 = powerof2 (64) };
_Static_assert (MIN (-2, 5) == -2 && MAX (-2, 5) == 5, "MIN, MAX constant");
_Static_assert (howmany (17, 8) == 3 && roundup (17, 8) == 24 && powerof2 (48) == 0, "howmany, roundup, powerof2 constant");
static char c_array[howmany (100, NBBY)];

int
main (void)
{
  char buf[16];
  volatile double nan = NAN, one = 1.0, pz = 0.0, nz = -0.0;
  int ia[4] = { 0 };
  signed char sc[2] = { 0 };

  S (C_MIN); S (C_MAX); S (C_HOWMANY); S (C_ROUNDUP); S (C_ROUNDUP2); S (C_POWEROF2); V (sizeof c_array);

  /* MIN, MAX: integer, unsigned, mixed sign, wider and narrower types, pointers, floating point. */
  S (MIN (3, 7)); S (MAX (3, 7)); S (MIN (-3, 7)); S (MAX (-3, -7));
  V (MIN (3u, 7u)); V (MAX (3u, UINT_MAX));
  V (MIN (-1, 1u)); V (MAX (-1, 1u)); S (MIN (-1, 1L)); S (MAX (-1L, 1u)); V (MIN (-1L, 1ul));
  S (MIN ((char) 1, (short) 2)); S (MAX ((unsigned char) 200, (signed char) -1));
  S (MIN (LLONG_MIN, 0)); V (MAX (0ull, ULLONG_MAX)); V (MIN ((size_t) 5, (size_t) 9)); S (MAX ((long) INT_MAX + 1, 0));
  S (MIN (buf + 3, buf + 9) - buf); S (MAX (buf + 3, buf + 9) - buf); printf ("MIN (buf, buf + 1) type\t%s\n", TYPE (MIN (buf, buf + 1)));
  D (MIN (1.5, 2)); D (MAX (1.5, 2)); D (MIN (2.5f, 1)); D (MAX (1, 0.5L));
  D (MIN (nan, one)); D (MIN (one, nan)); D (MAX (nan, one)); D (MAX (one, nan));
  D (MIN (pz, nz)); D (MIN (nz, pz)); D (MAX (pz, nz)); D (MAX (nz, pz));
  EVALS ("MIN a<b", 1, 2, MIN (A (), B ())); EVALS ("MIN a>b", 2, 1, MIN (A (), B ())); EVALS ("MIN a==b", 1, 1, MIN (A (), B ()));
  EVALS ("MAX a<b", 1, 2, MAX (A (), B ())); EVALS ("MAX a>b", 2, 1, MAX (A (), B ())); EVALS ("MAX a==b", 1, 1, MAX (A (), B ()));

  /* howmany: whole units of y needed for x. */
  S (howmany (0, 8)); S (howmany (1, 8)); S (howmany (8, 8)); S (howmany (9, 8)); S (howmany (10, 3)); S (howmany (9, 3));
  V (howmany (10u, 3u)); S (howmany (10L, 3)); V (howmany (10, 3u)); V (howmany ((size_t) 4097, (size_t) 4096));
  S (howmany (-5, 3)); S (howmany (-1, 8)); V (howmany (UINT_MAX, 2u)); V (howmany (ULLONG_MAX - 1, 2ull));
  S (howmany ((char) 9, (char) 4)); S (howmany (7, -2));
  EVALS ("howmany", 10, 3, howmany (A (), B ()));

  /* roundup: the least multiple of y not below x. */
  S (roundup (0, 8)); S (roundup (1, 8)); S (roundup (8, 8)); S (roundup (9, 8)); S (roundup (10, 3)); S (roundup (9, 3));
  V (roundup (10u, 3u)); S (roundup (10L, 4)); V (roundup (10, 4u)); V (roundup ((size_t) 4097, (size_t) 4096));
  S (roundup (-5, 3)); S (roundup (-5, 4)); S (roundup (-8, 4)); V (roundup (UINT_MAX, 2u)); V (roundup (UINT_MAX - 2u, 3u));
  V (roundup (0x7fffffffu, 0x40000000u)); S (roundup ((short) 9, (char) 4)); S (roundup (7, -2));
  EVALS ("roundup step 3", 10, 3, roundup (A (), B ())); EVALS ("roundup step 8", 10, 8, roundup (A (), B ()));
  EVALS ("roundup step 0x10 int", 10, 16, roundup (Ai (), Bi ())); EVALS ("roundup x multiple", 16, 8, roundup (A (), B ()));
  EVALS ("roundup constant step 8", 10, 0, roundup (A (), 8)); EVALS ("roundup constant step 3", 10, 0, roundup (A (), 3));
  /* Below zero, a power-of-two step known at compile time and one known only at run time; a narrower step. */
  RV ("roundup (-5, 4) run-time x", -5, 0, roundup (A (), 4)); RV ("roundup (-5, 4) run-time step", 0, 4, roundup (-5, B ()));
  RV ("roundup (-5, 4) both run-time", -5, 4, roundup (A (), B ())); RV ("roundup (-5, 3) run-time x", -5, 0, roundup (A (), 3));
  RV ("roundup (-5, 4) run-time int x", -5, 0, roundup (Ai (), 4)); RV ("roundup (-8, 4) run-time int x", -8, 0, roundup (Ai (), 4));
  S (roundup (5, 0)); S (roundup (-1, 0)); V (roundup (13, sizeof (long))); V (roundup (0x100000001ul, 8));
  V (roundup (0x100000001ul, 3u)); V (roundup (0x100000001ul, (unsigned char) 8));
  V (howmany (0x100000001ul, 8u)); S (howmany (5, 0 + 1)); S (roundup (9, 4 + 4)); V (roundup (9u, 1u << 3));

  /* powerof2: x & (x - 1) clear; 0 counts. */
  S (powerof2 (0)); S (powerof2 (1)); S (powerof2 (2)); S (powerof2 (3)); S (powerof2 (4)); S (powerof2 (6)); S (powerof2 (96));
  S (powerof2 (0x80000000u)); S (powerof2 (UINT_MAX)); S (powerof2 (1L << 40)); S (powerof2 ((1L << 40) + 1));
  S (powerof2 (-4)); S (powerof2 (-1)); S (powerof2 (-2)); S (powerof2 ((unsigned char) 128)); S (powerof2 (0ull));
  S (powerof2 (2.0 > 1)); EVALS ("powerof2 4", 4, 0, powerof2 (A ())); EVALS ("powerof2 6", 6, 0, powerof2 (A ()));
  EVALS ("powerof2 0", 0, 0, powerof2 (A ()));

  /* setbit, clrbit, isset, isclr: bit i of a bit array of NBBY-bit units. */
  S (NBBY);
  setbit (bits, 0); setbit (bits, 13); setbit (bits, 63);
  V (bits[0]); V (bits[1]); V (bits[7]); S (isset (bits, 0)); S (isset (bits, 13)); S (isset (bits, 63)); S (isset (bits, 12));
  S (isclr (bits, 0)); S (isclr (bits, 12)); S (isclr (bits, 13));
  S (setbit (bits, 9)); S (clrbit (bits, 13)); V (bits[1]); S (isset (bits, 13)); S (isclr (bits, 13));
  setbit (bits, 1u); setbit (bits, 2L); setbit (bits, (char) 3); V (bits[0]); clrbit (bits, 0); clrbit (bits, 1ul); V (bits[0]);
  setbit (bits, 7); S (isset (bits, 7)); V (bits[0]);
  setbit (ia, 13); setbit (ia, 33); S (ia[0]); S (ia[1]); S (ia[2]); S (ia[3]); S (isset (ia, 13)); S (isclr (ia, 13));
  setbit (sc, 7); S (sc[0]); S (isset (sc, 7)); S (isclr (sc, 7)); clrbit (sc, 7); S (sc[0]);
  EVALS ("setbit", 20, 0, setbit (Arr (), B ())); EVALS ("clrbit", 20, 0, clrbit (Arr (), B ()));
  EVALS ("isset", 20, 0, isset (Arr (), B ())); EVALS ("isclr", 20, 0, isclr (Arr (), B ()));
  return 0;
}
