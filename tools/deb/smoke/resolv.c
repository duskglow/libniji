/* libresolv through the packages (issue #5): a program linked with plain `gcc ... -lresolv` against
   libc6-dev needs libresolv.so.2 and binds ns_initparse and ns_parserr at GLIBC_2.9, as Debian's
   libkrb5.so.3 does. It parses a fixed response (RFC 1035 section 4.1: one question, one A record
   whose owner is a compression pointer to the question's name) and a message cut inside its
   header, and prints one line. Run by tools/deb/check-system.sh and tools/deb/root-system.sh. */
#include <errno.h>
#include <stdio.h>
#include <arpa/nameser.h>

static const unsigned char msg[] = {
    0x12, 0x34, 0x81, 0x80, 0, 1, 0, 1, 0, 0, 0, 0,
    1, 'a', 4, 't', 'e', 's', 't', 0, 0, 1, 0, 1,
    0xc0, 12, 0, 1, 0, 1, 0, 0, 0x0e, 0x10, 0, 4, 192, 0, 2, 1,
};

int main(void)
{
    ns_msg h;
    ns_rr rr;
    if (ns_initparse(msg, sizeof msg, &h) != 0 || ns_parserr(&h, ns_s_an, 0, &rr) != 0)
        return 1;
    const unsigned char *a = ns_rr_rdata(rr);
    errno = 0;
    int cut = ns_initparse(msg, 11, &h);
    printf("resolv: %s A %u.%u.%u.%u ttl %lu; cut header %d %s\n", ns_rr_name(rr), a[0], a[1], a[2], a[3],
           (unsigned long) ns_rr_ttl(rr), cut, errno == EMSGSIZE ? "EMSGSIZE" : "other");
    return 0;
}
