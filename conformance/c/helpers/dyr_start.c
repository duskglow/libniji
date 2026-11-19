/* Helper of s27_dynamic_address_entries (DL-DYNAMIC-RELOC): linked into the program, with a
 * DT_RUNPATH of /dyr/start (its HELPER-FLAGS line). Gives the test its own dynamic array, and the
 * array's DT_STRTAB entry as its constructor saw it. */
#include <elf.h>
#include <link.h>

extern ElfW(Dyn) _DYNAMIC[];

static ElfW(Addr) strtab_in_ctor;

__attribute__((constructor)) static void dyr_ctor(void)
{
    for (ElfW(Dyn) *d = _DYNAMIC; d->d_tag != DT_NULL; d++)
        if (d->d_tag == DT_STRTAB)
            strtab_in_ctor = d->d_un.d_ptr;
}

#ifndef DYR_PREFIX
#define DYR_PREFIX dyr_start
#endif
#define DYR_CAT2(a, b) a##b
#define DYR_CAT(a, b) DYR_CAT2(a, b)

ElfW(Dyn) *DYR_CAT(DYR_PREFIX, _dynamic)(void) { return _DYNAMIC; }
ElfW(Addr) DYR_CAT(DYR_PREFIX, _ctor_strtab)(void) { return strtab_in_ctor; }
