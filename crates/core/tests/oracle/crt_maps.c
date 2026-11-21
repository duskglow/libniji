/* CRT-START-FILES (GitHub #27): prints how many executable mappings of the program's own file the
 * process has (/proc/self/maps lines with permissions r-xp naming /proc/self/exe's target), the
 * count util-linux's lsfd `column-xmode` test relies on being one. crt_products.rs links it with each
 * start file. */
#include <stdio.h>
#include <string.h>
#include <unistd.h>

int main(void) {
    char exe[4096];
    ssize_t n = readlink("/proc/self/exe", exe, sizeof exe - 1);
    if (n < 0) {
        return 2;
    }
    exe[n] = 0;
    FILE *f = fopen("/proc/self/maps", "r");
    if (!f) {
        return 3;
    }
    char line[8192];
    int x = 0;
    while (fgets(line, sizeof line, f)) {
        char *path = strchr(line, '/');
        if (!path) {
            continue;
        }
        path[strcspn(path, "\n")] = 0;
        if (strcmp(path, exe) == 0 && strstr(line, " r-xp ")) {
            x++;
        }
    }
    fclose(f);
    printf("%d\n", x);
    return 0;
}
