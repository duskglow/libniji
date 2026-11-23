/* DL-DYNAMIC-RELOC black-box probe: for every object dl_iterate_phdr reports, compare each entry of
 * its in-memory dynamic section with the same entry in its file, and print which entries differ by
 * exactly the load base (A), are unchanged (=), or differ otherwise (?). Also: the view from a
 * constructor, _DYNAMIC, and the systemd pattern (DT_RUNPATH string through DT_STRTAB).
 *   probe DIR        DIR holds libdrB.so (dlopened). */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <elf.h>
#include <fcntl.h>
#include <link.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#ifdef STATIC_PROBE
static int dr_a(void) { return 42; }
#else
extern int dr_a(void);
#endif
static int phase;

static const char *tagname(Elf64_Sxword t) {
    switch (t) {
    case DT_NEEDED: return "NEEDED"; case DT_PLTRELSZ: return "PLTRELSZ"; case DT_PLTGOT: return "PLTGOT";
    case DT_HASH: return "HASH"; case DT_STRTAB: return "STRTAB"; case DT_SYMTAB: return "SYMTAB";
    case DT_RELA: return "RELA"; case DT_RELASZ: return "RELASZ"; case DT_RELAENT: return "RELAENT";
    case DT_STRSZ: return "STRSZ"; case DT_SYMENT: return "SYMENT"; case DT_INIT: return "INIT";
    case DT_FINI: return "FINI"; case DT_SONAME: return "SONAME"; case DT_RPATH: return "RPATH";
    case DT_SYMBOLIC: return "SYMBOLIC"; case DT_REL: return "REL"; case DT_DEBUG: return "DEBUG";
    case DT_TEXTREL: return "TEXTREL"; case DT_JMPREL: return "JMPREL"; case DT_BIND_NOW: return "BIND_NOW";
    case DT_INIT_ARRAY: return "INIT_ARRAY"; case DT_FINI_ARRAY: return "FINI_ARRAY";
    case DT_INIT_ARRAYSZ: return "INIT_ARRAYSZ"; case DT_FINI_ARRAYSZ: return "FINI_ARRAYSZ";
    case DT_RUNPATH: return "RUNPATH"; case DT_FLAGS: return "FLAGS"; case DT_PREINIT_ARRAY: return "PREINIT_ARRAY";
    case DT_PREINIT_ARRAYSZ: return "PREINIT_ARRAYSZ"; case DT_PLTREL: return "PLTREL";
    case 35: return "RELRSZ"; case 36: return "RELR"; case 37: return "RELRENT";
    case DT_GNU_HASH: return "GNU_HASH"; case DT_VERSYM: return "VERSYM"; case DT_RELACOUNT: return "RELACOUNT";
    case DT_FLAGS_1: return "FLAGS_1"; case DT_VERDEF: return "VERDEF"; case DT_VERDEFNUM: return "VERDEFNUM";
    case DT_VERNEED: return "VERNEED"; case DT_VERNEEDNUM: return "VERNEEDNUM"; case DT_AUXILIARY: return "AUXILIARY";
    case DT_FILTER: return "FILTER"; case DT_GNU_PRELINKED: return "GNU_PRELINKED";
    case DT_TLSDESC_PLT: return "TLSDESC_PLT"; case DT_TLSDESC_GOT: return "TLSDESC_GOT";
    default: return NULL;
    }
}

/* The file's dynamic entries (n of them) into out; 0 when the file cannot be read. */
static int file_dynamic(const char *path, Elf64_Dyn *out, int max) {
    int fd = open(path, O_RDONLY), n = 0;
    if (fd < 0) return 0;
    Elf64_Ehdr eh;
    if (pread(fd, &eh, sizeof eh, 0) == sizeof eh) {
        for (int i = 0; i < eh.e_phnum; i++) {
            Elf64_Phdr ph;
            if (pread(fd, &ph, sizeof ph, eh.e_phoff + (off_t)i * sizeof ph) != sizeof ph) break;
            if (ph.p_type != PT_DYNAMIC) continue;
            ssize_t got = pread(fd, out, ph.p_filesz < max * sizeof *out ? ph.p_filesz : max * sizeof *out, ph.p_offset);
            n = got > 0 ? (int)(got / sizeof *out) : 0;
        }
    }
    close(fd);
    return n;
}

static void show(const char *label, struct dl_phdr_info *info, const char *path) {
    const Elf64_Phdr *dynph = NULL;
    for (int i = 0; i < info->dlpi_phnum; i++)
        if (info->dlpi_phdr[i].p_type == PT_DYNAMIC) dynph = &info->dlpi_phdr[i];
    if (!dynph) { printf("%s: no PT_DYNAMIC\n", label); return; }
    int w = 0, relro = 0;
    for (int i = 0; i < info->dlpi_phnum; i++) {
        const Elf64_Phdr *p = &info->dlpi_phdr[i];
        int in = p->p_vaddr <= dynph->p_vaddr && dynph->p_vaddr < p->p_vaddr + p->p_memsz;
        if (in && p->p_type == PT_LOAD) w = !!(p->p_flags & PF_W);
        if (in && p->p_type == PT_GNU_RELRO) relro = 1;
    }
    const Elf64_Dyn *mem = (const Elf64_Dyn *)(info->dlpi_addr + dynph->p_vaddr);
    static Elf64_Dyn file[512];
    int nf = path ? file_dynamic(path, file, 512) : 0;
    printf("%s: base=%s W=%d RELRO=%d file=%s\n ", label, info->dlpi_addr ? "nonzero" : "0", w, relro, nf ? "read" : "none");
    for (int i = 0; mem[i].d_tag != DT_NULL; i++) {
        const char *nm = tagname(mem[i].d_tag);
        char hex[24];
        if (!nm) { snprintf(hex, sizeof hex, "%#lx", (unsigned long)mem[i].d_tag); nm = hex; }
        char c = '-';
        if (i < nf && file[i].d_tag == mem[i].d_tag) {
            Elf64_Xword d = mem[i].d_un.d_val - file[i].d_un.d_val;
            c = d == 0 ? '=' : d == info->dlpi_addr ? 'A' : '?';
        } else if (!nf) {
            c = mem[i].d_un.d_val >= info->dlpi_addr && info->dlpi_addr ? 'a' : 'r';
        }
        printf(" %s%c", nm, c);
    }
    printf("\n");
}

static int cb(struct dl_phdr_info *info, size_t size, void *data) {
    (void)size; (void)data;
    const char *n = info->dlpi_name;
    char label[96];
    const char *path = n;
    if (!n || !*n) { snprintf(label, sizeof label, "[%d] main", phase); path = "/proc/self/exe"; }
    else {
        const char *s = strrchr(n, '/');
        snprintf(label, sizeof label, "[%d] %s", phase, s ? s + 1 : n);
        if (strstr(n, "vdso")) path = NULL;
    }
    if (phase == 0 && strncmp(label + 4, "main", 4) != 0) return 0;
    show(label, info, path);
    return 0;
}

__attribute__((constructor)) static void early(void) {
    setvbuf(stdout, NULL, _IONBF, 0);
    phase = 0;
    dl_iterate_phdr(cb, NULL);
}

static void pre(void) {}
__attribute__((section(".preinit_array"), used)) static void (*pre_entry)(void) = pre;

int main(int argc, char **argv) {
    if (argc < 2) return 2;
    phase = 1;
    dl_iterate_phdr(cb, NULL);
#ifdef STATIC_PROBE
    return 0;
#endif
    /* _DYNAMIC and the systemd pattern: DT_RUNPATH's string through DT_STRTAB. */
    const char *strtab = NULL; long runpath = -1;
    for (const ElfW(Dyn) *d = _DYNAMIC; d->d_tag != DT_NULL; d++) {
        if (d->d_tag == DT_STRTAB) strtab = (const char *)d->d_un.d_ptr;
        if (d->d_tag == DT_RUNPATH) runpath = (long)d->d_un.d_val;
    }
    void *self = dlopen(NULL, RTLD_NOW);
    struct link_map *lm = NULL;
    dlinfo(self, RTLD_DI_LINKMAP, &lm);
    printf("_DYNAMIC: STRTAB-l_addr=%#lx l_ld==_DYNAMIC:%d\n", (unsigned long)((uintptr_t)strtab - lm->l_addr), (void *)lm->l_ld == (void *)_DYNAMIC);
    if (runpath >= 0 && (uintptr_t)strtab > lm->l_addr) printf("RUNPATH via STRTAB: \"%s\"\n", strtab + runpath);
    printf("dr_a=%d\n", dr_a());
    char p[4096];
    snprintf(p, sizeof p, "%s/libdrB.so", argv[1]);
    void *h = dlopen(p, RTLD_NOW);
    if (!h) { printf("dlopen B: %s\n", dlerror()); return 1; }
    phase = 2;
    dl_iterate_phdr(cb, NULL);
    return 0;
}
