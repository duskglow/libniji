@@if CTRL
/* CTRL: the control character typed with the key c (c & 037), an integer constant expression usable in #if, its
   argument evaluated once (R-H7: sysvinit sulogin, bsd-mailx). Each header that shows it carries this definition under
   one guard: the first one included defines it. */
# ifndef __NIJI_D_CTRL
#  define __NIJI_D_CTRL 1
#  define CTRL(__c) ((__c) & 037)
# endif
#endif
