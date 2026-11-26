@@if CCEQ
/* CCEQ: whether the control character c is the one val, a disabled character (_POSIX_VDISABLE, from <unistd.h>,
   which the program includes) matching none (R-H7: sysvinit sulogin). */
# define CCEQ(__v, __c) ((__c) == (__v) && (__v) != _POSIX_VDISABLE)
#endif
