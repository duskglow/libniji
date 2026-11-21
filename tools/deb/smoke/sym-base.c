/* tools/deb/check-symbols.sh sample 1: a program that uses only GLIBC_2.2.5 symbols. It has its
   own entry point (linked with -nostartfiles), so the start files' __libc_start_main@GLIBC_2.34 is
   not referenced: dpkg-shlibdeps must give libc6 (>= 2.2.5). Built, never run. */
#include <stdio.h>
#include <unistd.h>

void _start(void)
{
    puts("sample 1");
    _exit(0);
}
