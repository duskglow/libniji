@@owns ELF_NOTE_FDO ELF_NOTE_GNU ELF_NOTE_SOLARIS
/* ELF_NOTE_SOLARIS, ELF_NOTE_GNU, ELF_NOTE_FDO: the owner names (strings) of ELF notes. <elf.h> and <sys/auxv.h> show
   them; each carries this definition under one guard, so the first one included defines them (a program's earlier
   definition is replaced, as in the reference: black-box probe; R-H7). */
@@if ELF_NOTE_GNU
# ifndef __NIJI_D_elf_note_names
#  define __NIJI_D_elf_note_names 1
#  define ELF_NOTE_SOLARIS @@value(ELF_NOTE_SOLARIS)
#  define ELF_NOTE_GNU @@value(ELF_NOTE_GNU)
#  define ELF_NOTE_FDO @@value(ELF_NOTE_FDO)
# endif
#endif
