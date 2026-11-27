/* sched_setattr(2) and sched_getattr(2) (glibc 2.41 interface, taken early by owner ruling R3; spec section 24.3) with
   GNU extensions only. struct sched_attr is the kernel UAPI's own, from <linux/sched/types.h> (so a program that
   also includes that header, in either order, sees one definition, and one that defines its own struct sched_attr
   beside this header is a redefinition). The scheduling-policy and sched_flags names are spelled as the kernel's
   <linux/sched.h> spells them, so that header may come before or after this one without a redefinition warning;
   SCHED_FLAG_ALL is not defined here. Black-box probe of the 2.41 interface, C and C++, every configuration. */
#if __NIJI_GNU
# include <linux/sched/types.h>
# define SCHED_NORMAL		0
# define SCHED_EXT		7
# define SCHED_FLAG_RESET_ON_FORK	0x01
# define SCHED_FLAG_RECLAIM		0x02
# define SCHED_FLAG_DL_OVERRUN		0x04
# define SCHED_FLAG_KEEP_POLICY		0x08
# define SCHED_FLAG_KEEP_PARAMS		0x10
# define SCHED_FLAG_UTIL_CLAMP_MIN	0x20
# define SCHED_FLAG_UTIL_CLAMP_MAX	0x40
# define SCHED_FLAG_KEEP_ALL	(SCHED_FLAG_KEEP_POLICY | SCHED_FLAG_KEEP_PARAMS)
# define SCHED_FLAG_UTIL_CLAMP	(SCHED_FLAG_UTIL_CLAMP_MIN | SCHED_FLAG_UTIL_CLAMP_MAX)
extern int sched_setattr (__pid_t, struct sched_attr *, unsigned int) __NIJI_NOTHROW __attribute__ ((__nonnull__ (2)));
extern int sched_getattr (__pid_t, struct sched_attr *, unsigned int, unsigned int) __NIJI_NOTHROW
  __attribute__ ((__nonnull__ (2)));
#endif
