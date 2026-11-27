@@if va_list
# ifndef _VA_LIST_DEFINED
#  define __need___va_list
#  include <stdarg.h>
typedef __gnuc_va_list va_list;
#  define _VA_LIST_DEFINED
# endif
#endif
