@@owns-macro __iscanonical
@@if __iscanonical
/* Every encoding of float, double and binary128 is canonical. The x87 types (long double, _Float64x) have encodings
   that are not (pseudo-denormals, unnormals, pseudo-infinities, pseudo-NaNs): the library tests those bits. */
# define __iscanonical(x) (__builtin_types_compatible_p (__typeof (x), long double) \
			   || __builtin_types_compatible_p (__typeof (x), _Float64x) \
			   ? __iscanonicall ((long double) (x)) : ((void) (x), 1))
#endif
