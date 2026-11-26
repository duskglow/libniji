@@owns-macro isnan
/* isnan and isinf are also the functions of their own name (BSD), declared by @@emit above where
   visibility.tsv has them, before the macros: `&isinf` and `(isinf) (x)` reach the function. */
@@when isnan
# define isnan(x) __builtin_isnan (x)
#endif
