@@owns AUTHTYPE_NAME AUTHTYPE_NAME_OK ENCTYPE_NAME ENCTYPE_NAME_OK ENCRYPT_NAME ENCRYPT_NAME_OK SLC_NAME SLC_NAME_OK TELCMD TELCMD_OK SLC_NAMELIST SLC_NAMES authtype_names enctype_names encrypt_names slc_names telcmds
/* The printable names of the TELNET commands (RFC 854), options (RFC 855 and the option RFCs), line-mode special
   characters (RFC 1184), authentication types (RFC 2941) and encryption types and suboption commands (RFC 2946), for
   a client or server that traces its negotiation (inetutils' telnet and libtelnet). Each table is an array of
   `char *` ending in a null pointer. The tables are declared everywhere (telopts only with TELOPTS); a program
   defines one in exactly one translation unit by defining its switch before the header: AUTH_NAMES
   (authtype_names), ENCRYPT_NAMES (encrypt_names and enctype_names), TELCMDS (telcmds), TELOPTS (telopts, with
   TELOPT_FIRST, TELOPT_LAST, TELOPT_OK and TELOPT), SLC_NAMES (slc_names). The library itself defines none of them.
   A *_OK macro converts its argument to unsigned int, so a negative value is out of range; its result is a
   comparison (int in C, bool in C++), not usable in #if. TELCMD_OK evaluates its argument twice, the others once.
   Black-box probe of every configuration, C and C++. The terminating name is the integer constant 0 (D-26.3). */
extern char *authtype_names[];
extern char *encrypt_names[];
extern char *enctype_names[];
extern char *telcmds[];
extern char *slc_names[];

#define AUTHTYPE_NAME_OK(x) ((unsigned int) (x) < AUTHTYPE_CNT)
#define AUTHTYPE_NAME(x) authtype_names[x]
#define ENCRYPT_NAME_OK(x) ((unsigned int) (x) < ENCRYPT_CNT)
#define ENCRYPT_NAME(x) encrypt_names[x]
#define ENCTYPE_NAME_OK(x) ((unsigned int) (x) < ENCTYPE_CNT)
#define ENCTYPE_NAME(x) enctype_names[x]
#define TELCMD_OK(x) ((unsigned int) (x) <= TELCMD_LAST && (unsigned int) (x) >= TELCMD_FIRST)
#define TELCMD(x) telcmds[(x) - TELCMD_FIRST]
#define SLC_NAME_OK(x) ((unsigned int) (x) <= NSLC)
#define SLC_NAME(x) slc_names[x]
#define SLC_NAMELIST (char *) "0", (char *) "SYNCH", (char *) "BRK", (char *) "IP", (char *) "AO", (char *) "AYT", \
  (char *) "EOR", (char *) "ABORT", (char *) "EOF", (char *) "SUSP", (char *) "EC", (char *) "EL", (char *) "EW", \
  (char *) "RP", (char *) "LNEXT", (char *) "XON", (char *) "XOFF", (char *) "FORW1", (char *) "FORW2", 0

#ifdef AUTH_NAMES
char *authtype_names[] = {
  (char *) "NULL", (char *) "KERBEROS_V4", (char *) "KERBEROS_V5", (char *) "SPX", (char *) "MINK", 0
};
#endif

#ifdef ENCRYPT_NAMES
char *encrypt_names[] = {
  (char *) "IS", (char *) "SUPPORT", (char *) "REPLY", (char *) "START", (char *) "END", (char *) "REQUEST-START",
  (char *) "REQUEST-END", (char *) "ENC-KEYID", (char *) "DEC-KEYID", 0
};
char *enctype_names[] = {
  (char *) "ANY", (char *) "DES_CFB64", (char *) "DES_OFB64", 0
};
#endif

#ifdef TELCMDS
char *telcmds[] = {
  (char *) "EOF", (char *) "SUSP", (char *) "ABORT", (char *) "EOR", (char *) "SE", (char *) "NOP", (char *) "DMARK",
  (char *) "BRK", (char *) "IP", (char *) "AO", (char *) "AYT", (char *) "EC", (char *) "EL", (char *) "GA",
  (char *) "SB", (char *) "WILL", (char *) "WONT", (char *) "DO", (char *) "DONT", (char *) "IAC", 0
};
#endif

#ifdef TELOPTS
# define TELOPT_FIRST TELOPT_BINARY
# define TELOPT_LAST TELOPT_NEW_ENVIRON
# define TELOPT_OK(x) ((unsigned int) (x) <= TELOPT_LAST)
# define TELOPT(x) telopts[(x) - TELOPT_FIRST]
char *telopts[NTELOPTS + 1] = {
  (char *) "BINARY", (char *) "ECHO", (char *) "RCP", (char *) "SUPPRESS GO AHEAD", (char *) "NAME", (char *) "STATUS",
  (char *) "TIMING MARK", (char *) "RCTE", (char *) "NAOL", (char *) "NAOP", (char *) "NAOCRD", (char *) "NAOHTS",
  (char *) "NAOHTD", (char *) "NAOFFD", (char *) "NAOVTS", (char *) "NAOVTD", (char *) "NAOLFD",
  (char *) "EXTEND ASCII", (char *) "LOGOUT", (char *) "BYTE MACRO", (char *) "DATA ENTRY TERMINAL", (char *) "SUPDUP",
  (char *) "SUPDUP OUTPUT", (char *) "SEND LOCATION", (char *) "TERMINAL TYPE", (char *) "END OF RECORD",
  (char *) "TACACS UID", (char *) "OUTPUT MARKING", (char *) "TTYLOC", (char *) "3270 REGIME", (char *) "X.3 PAD",
  (char *) "NAWS", (char *) "TSPEED", (char *) "LFLOW", (char *) "LINEMODE", (char *) "XDISPLOC", (char *) "OLD-ENVIRON",
  (char *) "AUTHENTICATION", (char *) "ENCRYPT", (char *) "NEW-ENVIRON", 0
};
#endif

/* SLC_NAMES, the program's switch, is the list itself after the header (as SLC_NAMELIST), defined or not before. */
#ifdef SLC_NAMES
char *slc_names[] = { SLC_NAMELIST };
# undef SLC_NAMES
#endif
#define SLC_NAMES SLC_NAMELIST
