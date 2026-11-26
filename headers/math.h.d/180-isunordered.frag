@@owns-macro isunordered
@@if isunordered
# define isunordered(x, y) __builtin_isunordered (x, y)
#endif
