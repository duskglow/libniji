/* tools/deb/check-symbols.sh sample 4: a program that registers an exit handler at the Liella-only
   node LIELLA_OWNER_1 (atexit@LIELLA_OWNER_1, the opt-in of D-27.29): dpkg-shlibdeps must add
   liella-owner-abi (>= 1), the virtual package only Liella's libc6 provides. The opt-in's
   link-stub directory is not in the packages, so the sample binds the non-default version itself
   (.symver on an undefined name). Built, never run. */
#include <stdio.h>

int owner_atexit(void (*fn)(void));
__asm__(".symver owner_atexit, atexit@LIELLA_OWNER_1");

static void bye(void)
{
    puts("sample 4: exit handler");
}

int main(void)
{
    return owner_atexit(bye) != 0;
}
