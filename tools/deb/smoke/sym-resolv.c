/* tools/deb/check-symbols.sh sample 5: a program linked with -lresolv that uses ns_initparse and
   ns_parserr, the GLIBC_2.9 symbols Debian's libkrb5.so.3 needs from libresolv.so.2 (issue #5),
   and _exit@GLIBC_2.2.5. Its own entry point (-nostartfiles): dpkg-shlibdeps must give
   libc6 (>= 2.9) from libresolv.so.2's block of the symbols file. Built, never run. */
#include <arpa/nameser.h>
#include <unistd.h>

static const unsigned char msg[12];

void _start(void)
{
    ns_msg h;
    ns_rr rr;
    _exit(ns_initparse(msg, sizeof msg, &h) == 0 && ns_parserr(&h, ns_s_an, 0, &rr) == 0);
}
