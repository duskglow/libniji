@@if CTRL
/* CTRL: the control character typed with the key c (c & 037), an integer constant expression usable in #if, its
   argument evaluated once (R-H7: sysvinit sulogin, bsd-mailx). Each header that shows it defines it in the same
   words, so any of them may come first. */
# define CTRL(__c) ((__c) & 037)
#endif
