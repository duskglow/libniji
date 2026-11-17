/* P6 of slice 11 (plan/headers.md section 3): the type-generic macros of <tgmath.h> give the type the standard and
   GCC's __builtin_tgmath name, for float, double, long double, the _FloatN types (_Float128 included) and complex
   arguments, narrow arithmetic (fadd ... f64xfma) and integer arguments. A wrong list in a macro is a wrong type or an
   error here. Compiled by header_check.py in the `gnu` configuration. */
#include <stdint.h>
#include <tgmath.h>
#define IS(e, T) _Static_assert(_Generic((e), T: 1, default: 0), #e " is not " #T)
#define ALL(MAC, EXPR_T) \
  EXPR_T(MAC, float); EXPR_T(MAC, double); EXPR_T(MAC, long double); \
  EXPR_T(MAC, _Float32); EXPR_T(MAC, _Float64); EXPR_T(MAC, _Float32x); EXPR_T(MAC, _Float64x); EXPR_T(MAC, _Float128)
#define U(MAC, T) IS(MAC((T) 1), T)
#define B(MAC, T) IS(MAC((T) 1, (T) 1), T)
#define C(MAC, T) IS(MAC((T) 1, (T) 1, (T) 1), T)
#define CU(MAC, T) IS(MAC((T _Complex) 1), T _Complex)
#define CR(MAC, T) IS(MAC((T _Complex) 1), T)
ALL(sqrt, U); ALL(sqrt, CU); ALL(fabs, U); ALL(fabs, CR); ALL(exp, U); ALL(exp, CU); ALL(cbrt, U);
ALL(pow, B); ALL(atan2, B); ALL(fmax, B); ALL(fma, C); ALL(exp10, U); ALL(fmaximum_num, B); ALL(nextup, U);
ALL(carg, CR); ALL(creal, CR); ALL(conj, CU); ALL(cproj, CU);
IS(carg((float) 1), float); IS(carg((double) 1), double); IS(creal((long double) 1), long double);
IS(ldexp((float) 1, 2), float); IS(lrint((_Float128) 1), long); IS(ilogb(1.0L), int);
IS(nexttoward(1.0f, 2.0L), float); IS(fromfp(1.0, 0, 8), intmax_t);
IS(fadd(1.0, 2.0), float); IS(fadd(1.0L, 2.0L), float); IS(fadd(1.0f, 2.0f), float); IS(dadd(1.0L, 1.0L), double);
IS(dadd(1.0f, 1.0), double); IS(dsqrt(2.0L), double); IS(ffma(1.0, 2.0, 3.0), float);
IS(f32add((_Float64) 1, (_Float64) 2), _Float32); IS(f32add((_Float128) 1, (_Float128) 2), _Float32);
IS(f64xadd((_Float128) 1, (_Float128) 2), _Float64x); IS(f32xsqrt((_Float64) 2), _Float32x); IS(f64fma((_Float128) 1, 1, 1), _Float64);
IS(sin(1) , double); IS(sqrt(1), double); IS(fabs((short) 1), double);
IS(log(1.0 + 1.0 * I), double _Complex); IS(pow(1.0f, 2.0), double);
int main(void) { return 0; }
