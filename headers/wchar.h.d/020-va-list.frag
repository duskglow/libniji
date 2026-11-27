@@if va_list
# ifndef __NIJI_D_va_list
#  define __NIJI_D_va_list 1
#  define __need___va_list
#  include <stdarg.h>
typedef __gnuc_va_list va_list;
# endif
#endif
