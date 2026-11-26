@@owns __GNU_GETTEXT_SUPPORTED_REVISION
/* The gettext interface revision this C library supports for a major revision (gettext.m4's AM_GNU_GETTEXT test
   for "GNU gettext in libc"): minor revision 1 of major revision 0 (ngettext, dngettext and dcngettext with plural
   forms, and the system-dependent format strings of a .mo file, `<PRId64>` and the like), -1 for any other major
   revision. An int constant expression of a constant argument, which is evaluated once. */
@@if __GNU_GETTEXT_SUPPORTED_REVISION
# define __GNU_GETTEXT_SUPPORTED_REVISION(major) ((major) == 0 ? 1 : -1)
#endif

