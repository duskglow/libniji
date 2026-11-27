/* The printable names of RFC 959's representation types (TYPE_A .. TYPE_L), file structures (STRU_F .. STRU_P),
   transfer modes (MODE_S .. MODE_C) and format controls (FORM_N .. FORM_C), indexed by those codes (they start at
   1; entry 0 is the string "0"), for an FTP server's replies (inetutils' ftpd). A program defines them in exactly
   one translation unit by defining FTP_NAMES (with any value or none) before the header: each is then an array of
   `char *` with external linkage and no terminating null pointer. Without FTP_NAMES none of them is declared. The
   library itself defines none of them. Black-box probe of every configuration, C and C++. */
#ifdef FTP_NAMES
char *modenames[] = { (char *) "0", (char *) "Stream", (char *) "Block", (char *) "Compressed" };
char *strunames[] = { (char *) "0", (char *) "File", (char *) "Record", (char *) "Page" };
char *typenames[] = { (char *) "0", (char *) "ASCII", (char *) "EBCDIC", (char *) "Image", (char *) "Local" };
char *formnames[] = { (char *) "0", (char *) "Nonprint", (char *) "Telnet", (char *) "Carriage-control" };
#endif
