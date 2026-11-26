/* SYSLOG_NAMES, defined by the program before the header: the level and facility names of syslog.conf(5) and their
   values, as two tables each ending in a null name with value -1, for a program that parses or prints them (a logging
   daemon, logger(1)). They are definitions, so exactly one translation unit of a program may define SYSLOG_NAMES. The
   terminating name is the integer constant 0, a null pointer constant in C and C++ alike, so the header needs no
   <stddef.h> (D-26.3). "none" (no level) and "mark" (the daemon's own timestamp facility, one past the last facility)
   have values outside the levels and facilities, INTERNAL_NOPRI and INTERNAL_MARK. Only where the BSD names are
   visible (_DEFAULT_SOURCE, not the strict standards or _XOPEN_SOURCE alone: black-box probe). */
#if __NIJI_MISC
# ifdef SYSLOG_NAMES
#  define INTERNAL_NOPRI 0x10
#  define INTERNAL_MARK LOG_MAKEPRI(LOG_NFACILITIES << 3, 0)
typedef struct _code {
  char *c_name;
  int c_val;
} CODE;
CODE prioritynames[] = {
  { "alert", LOG_ALERT },
  { "crit", LOG_CRIT },
  { "debug", LOG_DEBUG },
  { "emerg", LOG_EMERG },
  { "err", LOG_ERR },
  { "error", LOG_ERR },
  { "info", LOG_INFO },
  { "none", INTERNAL_NOPRI },
  { "notice", LOG_NOTICE },
  { "panic", LOG_EMERG },
  { "warn", LOG_WARNING },
  { "warning", LOG_WARNING },
  { 0, -1 }
};
CODE facilitynames[] = {
  { "auth", LOG_AUTH },
  { "authpriv", LOG_AUTHPRIV },
  { "cron", LOG_CRON },
  { "daemon", LOG_DAEMON },
  { "ftp", LOG_FTP },
  { "kern", LOG_KERN },
  { "lpr", LOG_LPR },
  { "mail", LOG_MAIL },
  { "mark", INTERNAL_MARK },
  { "news", LOG_NEWS },
  { "security", LOG_AUTH },
  { "syslog", LOG_SYSLOG },
  { "user", LOG_USER },
  { "uucp", LOG_UUCP },
  { "local0", LOG_LOCAL0 },
  { "local1", LOG_LOCAL1 },
  { "local2", LOG_LOCAL2 },
  { "local3", LOG_LOCAL3 },
  { "local4", LOG_LOCAL4 },
  { "local5", LOG_LOCAL5 },
  { "local6", LOG_LOCAL6 },
  { "local7", LOG_LOCAL7 },
  { 0, -1 }
};
# endif
#endif
