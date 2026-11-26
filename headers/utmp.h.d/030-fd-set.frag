@@owns FD_CLR FD_ISSET FD_ZERO
/* select(2): the fd_set macros, as <sys/types.h> has them, in the modes where this header shows fd_set and that one does not. */
@@if FD_SET
# ifndef __NIJI_FD_BITS
/* The word index goes through the library's range check (an out-of-range descriptor aborts instead of writing outside the set). */
extern long int __fdelt_chk (long int) __NIJI_NOTHROW;
#  define __NIJI_FD_BITS(s) (8 * (int) sizeof (*(s)->fds_bits))
#  define FD_SET(fd, s) ((void) ((s)->fds_bits[__fdelt_chk (fd)] |= (__typeof__ (*(s)->fds_bits)) 1 << ((fd) % __NIJI_FD_BITS (s))))
#  define FD_CLR(fd, s) ((void) ((s)->fds_bits[__fdelt_chk (fd)] &= ~((__typeof__ (*(s)->fds_bits)) 1 << ((fd) % __NIJI_FD_BITS (s)))))
#  define FD_ISSET(fd, s) (((s)->fds_bits[__fdelt_chk (fd)] & ((__typeof__ (*(s)->fds_bits)) 1 << ((fd) % __NIJI_FD_BITS (s)))) != 0)
#  define FD_ZERO(s) ((void) __builtin_memset ((s), 0, sizeof (*(s))))
# endif
#endif
