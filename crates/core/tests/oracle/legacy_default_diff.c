/* `bindresvport` and `__libc_sa_len` linked by name (their default versions, no compat alias):
 * prints every family `__libc_sa_len` knows and `bindresvport`'s result and errno for the
 * failures every caller can reach (a non-AF_INET `sin`, an AF_INET6 socket, a bad descriptor)
 * and for a plain call, which succeeds only with the privilege to bind a reserved port. The
 * output must be the reference's, line for line (legacy_default_products.rs). */
#include <errno.h>
#include <netinet/in.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

/* No installed header declares it. */
extern int __libc_sa_len(sa_family_t af);

static void call(const char *what, int fd, struct sockaddr_in *sin) {
    errno = 0;
    int rc = bindresvport(fd, sin);
    int e = errno;
    if (rc == 0) {
        struct sockaddr_in got;
        socklen_t l = sizeof got;
        memset(&got, 0, sizeof got);
        getsockname(fd, (struct sockaddr *)&got, &l);
        unsigned p = ntohs(got.sin_port);
        printf("bindresvport %s: rc=0 in_range=%d sin_updated=%d\n", what, p >= 512 && p <= 1023,
               sin == NULL || sin->sin_port == got.sin_port);
    } else {
        printf("bindresvport %s: rc=%d errno=%d\n", what, rc, e);
    }
}

int main(void) {
    int known = 0;
    for (unsigned af = 0; af < 65536; af++) {
        int n = __libc_sa_len((sa_family_t)af);
        if (n != 0) {
            printf("__libc_sa_len(%u) = %d\n", af, n);
            known++;
        }
    }
    printf("__libc_sa_len: %d families known\n", known);

    struct sockaddr_in sin;
    int s = socket(AF_INET, SOCK_STREAM, 0);
    call("NULL", s, NULL);
    close(s);
    s = socket(AF_INET, SOCK_STREAM, 0);
    memset(&sin, 0, sizeof sin);
    sin.sin_family = AF_INET;
    sin.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    call("sin AF_INET", s, &sin);
    close(s);
    s = socket(AF_INET, SOCK_STREAM, 0);
    memset(&sin, 0, sizeof sin);
    sin.sin_family = AF_INET6;
    call("sin AF_INET6", s, &sin);
    sin.sin_family = AF_UNSPEC;
    call("sin AF_UNSPEC", s, &sin);
    close(s);
    s = socket(AF_INET6, SOCK_STREAM, 0);
    call("AF_INET6 socket", s, NULL);
    close(s);
    call("bad descriptor", 999, NULL);
    return 0;
}
