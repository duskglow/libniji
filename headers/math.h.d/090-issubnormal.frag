@@owns-macro issubnormal
@@if issubnormal
# define issubnormal(x) (__builtin_fpclassify (FP_NAN, FP_INFINITE, FP_NORMAL, FP_SUBNORMAL, FP_ZERO, x) == FP_SUBNORMAL)
#endif
