@@owns LOG_PRI LOG_FAC LOG_MAKEPRI
/* A priority value is a facility (already shifted, LOG_USER is 1 << 3) or'ed with a level: its level, its facility
   number, and the value of a pair. */
@@when LOG_MAKEPRI
# define LOG_PRI(p) ((p) & LOG_PRIMASK)
# define LOG_FAC(p) (((p) & LOG_FACMASK) >> 3)
# define LOG_MAKEPRI(fac, pri) ((fac) | (pri))
#endif

