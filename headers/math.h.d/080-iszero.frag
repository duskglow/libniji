@@owns-macro iszero
@@if iszero
# define iszero(x) (__builtin_fpclassify (FP_NAN, FP_INFINITE, FP_NORMAL, FP_SUBNORMAL, FP_ZERO, x) == FP_ZERO)
#endif
