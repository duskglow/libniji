@@if-probe CMPLXF32X cabsf32x
# define CMPLXF32X(x, y) __builtin_complex ((_Float32x) (x), (_Float32x) (y))
#endif
