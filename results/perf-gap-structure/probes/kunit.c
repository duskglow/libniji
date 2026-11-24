/* PERF-GAP-STRUCTURE-AUDIT kernel unit costs: kunit <mode> <n> [file]. Per unit = (count(n) - count(0)) / n.
 * anon: touch n fresh anonymous pages (zero-fill faults). fileread64k: read one byte every 64 KiB of a mapped
 * file (one fault-around read fault each). cow: write one byte to each of n pages of a private file mapping
 * (copy-on-write faults). mmapfile: n x (mmap one page of the file, munmap). */
#define _GNU_SOURCE
#include <fcntl.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>

int main(int argc, char **argv)
{
    if (argc < 4)
        return 2;
    const char *m = argv[1];
    long n = atol(argv[2]);
    int fd = open(argv[3], O_RDONLY);
    if (fd < 0)
        return 3;
    volatile unsigned long sum = 0;
    if (!strcmp(m, "anon")) {
        char *p = mmap(NULL, (size_t)(n + 1) * 4096, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
        if (p == MAP_FAILED)
            return 4;
        for (long i = 0; i < n; i++)
            p[i * 4096] = 1;
    } else if (!strcmp(m, "fileread64k")) {
        char *p = mmap(NULL, (size_t)(n + 1) * 65536, PROT_READ, MAP_PRIVATE, fd, 0);
        if (p == MAP_FAILED)
            return 4;
        for (long i = 0; i < n; i++)
            sum += (unsigned char)p[i * 65536];
    } else if (!strcmp(m, "cow")) {
        char *p = mmap(NULL, (size_t)(n + 1) * 4096, PROT_READ | PROT_WRITE, MAP_PRIVATE, fd, 0);
        if (p == MAP_FAILED)
            return 4;
        for (long i = 0; i < n; i++)
            p[i * 4096] ^= 1;
    } else if (!strcmp(m, "mmapfile")) {
        for (long i = 0; i < n; i++) {
            void *p = mmap(NULL, 4096, PROT_READ, MAP_PRIVATE, fd, 0);
            if (p == MAP_FAILED)
                return 4;
            munmap(p, 4096);
        }
    } else {
        return 5;
    }
    return (int)(sum & 0);
}
