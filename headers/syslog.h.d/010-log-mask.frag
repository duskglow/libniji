@@owns LOG_MASK LOG_UPTO
/* syslog(3): the mask of one priority, and of all priorities up to one. */
@@when LOG_MASK
# define LOG_MASK(pri) (1 << (pri))
# define LOG_UPTO(pri) ((1 << ((pri) + 1)) - 1)
#endif

