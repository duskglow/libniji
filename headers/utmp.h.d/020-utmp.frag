@@owns ut_name ut_time ut_xtime ut_addr
/* utmp(5): the older names of struct utmp's members, macros for the members that replaced them (the user name, the
   entry time's seconds, the first word of the remote host's address). */
@@when utmp
# define ut_name ut_user
# define ut_time ut_tv.tv_sec
# define ut_xtime ut_tv.tv_sec
# define ut_addr ut_addr_v6[0]
#endif

