@@owns-macro isinf
@@when isinf
/* C 7.12.3.3 asks for non-zero; the sign (-1 for -inf, +1 for +inf) is the documented extension of the builtin. */
# define isinf(x) __builtin_isinf_sign (x)
#endif
