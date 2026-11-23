/* KERNEL-RESIDUAL-PROFILE: an empty main that prints its own VMAs and memory status at main.
   Built with each kit; the output is what the loader and the C library's initialisation left behind. */
#include <fcntl.h>
#include <unistd.h>

static char buf[65536];

static void dump(const char *path) {
    int fd = open(path, O_RDONLY);
    if (fd < 0)
        return;
    long n, off = 0;
    while ((n = read(fd, buf + off, sizeof buf - 1 - (unsigned long)off)) > 0)
        off += n;
    close(fd);
    write(1, "== ", 3);
    for (const char *p = path; *p; p++)
        write(1, p, 1);
    write(1, "\n", 1);
    write(1, buf, (unsigned long)off);
}

int main(void) {
    dump("/proc/self/smaps");
    dump("/proc/self/status");
    return 0;
}
