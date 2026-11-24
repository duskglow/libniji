/* DECISION-EVIDENCE-AUDIT probe: a process's memory with no allocation (argv[1] = 0) or one 16-byte malloc (1). */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(int argc, char **argv) {
    volatile char *p = 0;
    if (argc > 1 && argv[1][0] == '1') {
        p = malloc(16);
        p[0] = 1;
    }
    FILE *f = fopen("/proc/self/status", "r");
    char line[256];
    while (f && fgets(line, sizeof line, f))
        if (!strncmp(line, "VmSize", 6) || !strncmp(line, "VmRSS", 5) || !strncmp(line, "RssAnon", 7))
            fputs(strtok(line, "\n"), stdout), fputc(' ', stdout);
    return p ? 0 : 0;
}
