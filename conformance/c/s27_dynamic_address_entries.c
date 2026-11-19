/* Spec §27.1 (s27_1_c_8, dl_iterate_phdr: each loaded object's load address and program headers)
 * and the probe-settled content of the dynamic array they lead to (DL-DYNAMIC-RELOC, probe
 * tools/probe/dl/dynreloc, spec/behaviour/dynlink.md "The loaded dynamic array"): once the loader
 * has relocated an object whose dynamic array lies in a writable segment, the array's DT_PLTGOT,
 * DT_HASH, DT_STRTAB, DT_SYMTAB, DT_RELA, DT_JMPREL, DT_RELR, DT_GNU_HASH and DT_VERSYM entries
 * hold run-time addresses (the load bias added); DT_INIT_ARRAY, DT_FINI_ARRAY, DT_VERNEED and the
 * other entries keep their link-time values. Constructors already see run-time addresses.
 *
 * Checked through two views of the same array, `_DYNAMIC` (each object's own) and the PT_DYNAMIC
 * that dl_iterate_phdr leads to (and `l_ld` of a dlopen handle): for the program, a start-time
 * library (dyr_start), a dlopen'ed library (dyr_open) and every other object listed but the vDSO.
 * The use programs make of it: find DT_RUNPATH (DT_NEEDED for the program) and read its string
 * at DT_STRTAB + d_val. Each failed check prints its line; the exit status is the number of
 * failed checks. */
/* LINK-HELPERS: dyr_start */
/* DLOPEN-HELPERS: dyr_open */
/* HELPER-FLAGS: dyr_start -Wl,--enable-new-dtags,-rpath,/dyr/start */
/* HELPER-FLAGS: dyr_open -Wl,--enable-new-dtags,-rpath,/dyr/open */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <elf.h>
#include <link.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/auxv.h>

static int fails;
#define CHECK(c) do { if (!(c)) { fprintf(stderr, "s27_dynamic_address_entries: line %d: %s\n", __LINE__, #c); fails++; } } while (0)
#define CHECK_IN(what, c) do { if (!(c)) { fprintf(stderr, "s27_dynamic_address_entries: %s: line %d: %s\n", what, __LINE__, #c); fails++; } } while (0)

extern ElfW(Dyn) _DYNAMIC[];
ElfW(Dyn) *dyr_start_dynamic(void);
ElfW(Addr) dyr_start_ctor_strtab(void);

/* One object as dl_iterate_phdr lists it. */
struct obj {
    ElfW(Addr) base;
    const ElfW(Phdr) *phdr;
    ElfW(Half) phnum;
};

/* Whether run-time address A lies inside one of the object's PT_LOADs. */
static int in_image(const struct obj *o, ElfW(Addr) a)
{
    for (ElfW(Half) i = 0; i < o->phnum; i++) {
        const ElfW(Phdr) *p = &o->phdr[i];
        if (p->p_type == PT_LOAD && a >= o->base + p->p_vaddr && a < o->base + p->p_vaddr + p->p_memsz)
            return 1;
    }
    return 0;
}

static ElfW(Dyn) *dynamic_of(const struct obj *o)
{
    for (ElfW(Half) i = 0; i < o->phnum; i++)
        if (o->phdr[i].p_type == PT_DYNAMIC)
            return (ElfW(Dyn) *)(o->base + o->phdr[i].p_vaddr);
    return NULL;
}

/* Whether the object's dynamic array lies wholly inside one writable PT_LOAD. */
static int dynamic_writable(const struct obj *o)
{
    const ElfW(Phdr) *d = NULL;
    for (ElfW(Half) i = 0; i < o->phnum; i++)
        if (o->phdr[i].p_type == PT_DYNAMIC)
            d = &o->phdr[i];
    for (ElfW(Half) i = 0; d != NULL && i < o->phnum; i++) {
        const ElfW(Phdr) *p = &o->phdr[i];
        if (p->p_type == PT_LOAD && (p->p_flags & PF_W) && d->p_vaddr >= p->p_vaddr && d->p_vaddr + d->p_memsz <= p->p_vaddr + p->p_memsz)
            return 1;
    }
    return 0;
}

/* The address entries are run-time addresses inside the object, the link-time ones are offsets
 * from its base; WANT_RUNPATH (or, for the program, a DT_NEEDED ending in WANT_NEEDED) is the
 * string at DT_STRTAB + d_val. Returns DT_STRTAB's value. */
static ElfW(Addr) check_array(const char *what, const struct obj *o, const char *want_runpath, const char *want_needed)
{
    ElfW(Dyn) *dyn = dynamic_of(o);
    CHECK_IN(what, dyn != NULL);
    if (dyn == NULL)
        return 0;
    ElfW(Addr) strtab = 0;
    int addresses = 0, runpath_ok = want_runpath == NULL, needed_ok = want_needed == NULL;
    for (ElfW(Dyn) *d = dyn; d->d_tag != DT_NULL; d++)
        if (d->d_tag == DT_STRTAB)
            strtab = d->d_un.d_ptr;
    for (ElfW(Dyn) *d = dyn; d->d_tag != DT_NULL; d++) {
        ElfW(Addr) v = d->d_un.d_ptr;
        switch (d->d_tag) {
        case DT_STRTAB: case DT_SYMTAB: case DT_HASH: case DT_GNU_HASH: case DT_VERSYM:
        case DT_RELA: case DT_JMPREL: case DT_PLTGOT:
            CHECK_IN(what, in_image(o, v));
            addresses++;
            break;
        case DT_INIT_ARRAY: case DT_FINI_ARRAY: case DT_VERNEED:
            CHECK_IN(what, in_image(o, o->base + v));
            break;
        case DT_RUNPATH:
            /* Read only through a DT_STRTAB inside the object: a link-time one is a failed check above. */
            if (want_runpath != NULL && in_image(o, strtab) && strcmp((const char *)strtab + v, want_runpath) == 0)
                runpath_ok = 1;
            break;
        case DT_NEEDED:
            if (want_needed != NULL && in_image(o, strtab)) {
                const char *s = (const char *)strtab + v;
                size_t n = strlen(s), w = strlen(want_needed);
                needed_ok |= n >= w && strcmp(s + n - w, want_needed) == 0;
            }
            break;
        }
    }
    CHECK_IN(what, addresses >= 3);
    CHECK_IN(what, runpath_ok);
    CHECK_IN(what, needed_ok);
    return strtab;
}

/* dl_iterate_phdr: the object whose PT_DYNAMIC is at WANT; every listed object but the vDSO checked. */
struct find {
    ElfW(Dyn) *want;
    struct obj found;
    int hits, others;
};

static int each(struct dl_phdr_info *info, size_t size, void *data)
{
    struct find *f = data;
    struct obj o = { info->dlpi_addr, info->dlpi_phdr, info->dlpi_phnum };
    (void)size;
    if (dynamic_of(&o) == f->want) {
        f->found = o;
        f->hits++;
    }
    const ElfW(Ehdr) *vdso = (const ElfW(Ehdr) *)getauxval(AT_SYSINFO_EHDR);
    int is_vdso = vdso != NULL && (const char *)info->dlpi_phdr == (const char *)vdso + vdso->e_phoff;
    if (!is_vdso && dynamic_of(&o) != NULL && dynamic_writable(&o)) {
        char what[256];
        snprintf(what, sizeof what, "listed object %s", info->dlpi_name);
        check_array(what, &o, NULL, NULL);
        f->others++;
    }
    return 0;
}

static struct obj listed(const char *what, ElfW(Dyn) *dyn)
{
    struct find f = { dyn, { 0, NULL, 0 }, 0, 0 };
    dl_iterate_phdr(each, &f);
    CHECK_IN(what, f.hits == 1);
    CHECK_IN(what, f.others >= 4);
    return f.found;
}

/* The program's DT_STRTAB entry as its own constructor saw it. */
static ElfW(Addr) prog_ctor_strtab;

__attribute__((constructor)) static void prog_ctor(void)
{
    for (ElfW(Dyn) *d = _DYNAMIC; d->d_tag != DT_NULL; d++)
        if (d->d_tag == DT_STRTAB)
            prog_ctor_strtab = d->d_un.d_ptr;
}

int main(void)
{
    const char *dir = getenv("CONF_HELPERS");
    if (dir == NULL) {
        fprintf(stderr, "s27_dynamic_address_entries: CONF_HELPERS unset\n");
        return 99;
    }

    /* The program: `_DYNAMIC` is the array dl_iterate_phdr leads to. */
    struct obj prog = listed("program", _DYNAMIC);
    CHECK(prog.phdr != NULL);
    if (prog.phdr != NULL) {
        ElfW(Addr) s = check_array("program", &prog, NULL, "libdyr_start.so");
        CHECK(s == prog_ctor_strtab);
    }

    /* A start-time library. */
    struct obj start = listed("dyr_start", dyr_start_dynamic());
    CHECK(start.phdr != NULL);
    if (start.phdr != NULL) {
        ElfW(Addr) s = check_array("dyr_start", &start, "/dyr/start", NULL);
        CHECK(s == dyr_start_ctor_strtab());
    }

    /* A dlopen'ed library; its link map's `l_ld` and `l_addr` are the same array and base. */
    char path[4096];
    snprintf(path, sizeof path, "%s/libdyr_open.so", dir);
    void *h = dlopen(path, RTLD_NOW);
    CHECK(h != NULL);
    if (h == NULL) {
        fprintf(stderr, "s27_dynamic_address_entries: dlopen %s: %s\n", path, dlerror());
        return fails;
    }
    ElfW(Dyn) *(*open_dynamic)(void) = (ElfW(Dyn) * (*)(void)) dlsym(h, "dyr_open_dynamic");
    ElfW(Addr) (*open_ctor)(void) = (ElfW(Addr)(*)(void))dlsym(h, "dyr_open_ctor_strtab");
    CHECK(open_dynamic != NULL && open_ctor != NULL);
    if (open_dynamic == NULL || open_ctor == NULL)
        return fails;
    struct obj opened = listed("dyr_open", open_dynamic());
    CHECK(opened.phdr != NULL);
    if (opened.phdr != NULL) {
        ElfW(Addr) s = check_array("dyr_open", &opened, "/dyr/open", NULL);
        CHECK(s == open_ctor());
        struct link_map *lm = NULL;
        CHECK(dlinfo(h, RTLD_DI_LINKMAP, &lm) == 0 && lm != NULL);
        if (lm != NULL) {
            CHECK(lm->l_ld == open_dynamic());
            CHECK(lm->l_addr == opened.base);
        }
    }
    dlclose(h);
    return fails;
}
