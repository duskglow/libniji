@@owns-macro TIMESPEC_TO_TIMEVAL TIMEVAL_TO_TIMESPEC
/* struct timeval <-> struct timespec: the same text as <sys/time.h>. */
@@when TIMESPEC_TO_TIMEVAL
# define TIMEVAL_TO_TIMESPEC(tv, ts) do { (ts)->tv_sec = (tv)->tv_sec; (ts)->tv_nsec = (tv)->tv_usec * 1000; } while (0)
# define TIMESPEC_TO_TIMEVAL(tv, ts) do { (tv)->tv_sec = (ts)->tv_sec; (tv)->tv_usec = (ts)->tv_nsec / 1000; } while (0)
#endif

