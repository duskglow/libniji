@@owns-macro __tolower_l __toupper_l
/* __tolower_l and __toupper_l: declared functions (the exports tolower_l and toupper_l call under _FORTIFY_SOURCE;
   R-H7: postgresql-17), and under _FORTIFY_SOURCE also macros that call them, as the reference's header shows them. */
@@when __tolower_l
# define __tolower_l(c, l) __tolower_l ((c), (l))
# define __toupper_l(c, l) __toupper_l ((c), (l))
#endif
