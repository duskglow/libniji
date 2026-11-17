/* tools/deb/check-symbols.sh sample 6: sample 1 linked with packed relative relocations
   (-z pack-relative-relocs). The pointer table needs relative relocations, so the link editor packs
   them (DT_RELR) and records a requirement on libc.so.6's GLIBC_ABI_DT_RELR, a version node no symbol
   carries. dpkg-shlibdeps derives shlibs:Depends from imported symbols only: libc6 (>= 2.2.5).
   Built, never run. */
#include <stdio.h>
#include <unistd.h>

static const char *const names[] = { "sample 6a", "sample 6b" };

void _start(void)
{
    puts(names[getpid() & 1]);
    _exit(0);
}
