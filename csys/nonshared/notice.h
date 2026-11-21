/* Class nonshared: the opt-in notice of the registration members of libc_nonshared.a (D-27.29,
 * owner ruling #4861). Off by default: the default archive carries neither section, and a link
 * against it prints nothing. A distribution that wants its packages moved to the opt-in
 * (the niji-owner link-stub directory, NIJI_OWNER_1) builds the archive with
 * `NIJI_NONSHARED_NOTICE=1 cargo xtask so`, which compiles each registration member with
 * -DNIJI_NONSHARED_NOTICE. The member then carries:
 *
 *  - `.gnu.warning.<name>`: the link editor prints its text when a module's reference to <name>
 *    is resolved to this member (GNU ld, "Special sections");
 *  - `.note.niji.nonshared`: an allocated note (owner "libniji", type 1, empty descriptor),
 *    kept under --gc-sections, so it ends up in the linked module's PT_NOTE. libniji's ld.so names
 *    every loaded module that carries it when NIJI_LD_REPORT_NONSHARED is set (not in secure
 *    mode).
 *
 * Nothing changes at run time: the member registers exactly as without the notice. */

#ifndef NIJI_NONSHARED_NOTICE_H
#define NIJI_NONSHARED_NOTICE_H

#ifdef NIJI_NONSHARED_NOTICE
#define NIJI_NONSHARED_NOTICE_FOR(name)                                                              \
    __asm__(".pushsection .gnu.warning." #name ",\"\",@progbits\n\t"                                  \
            ".string \"" #name " is linked from libc_nonshared.a; link with the niji-owner "        \
            "directory to register at NIJI_OWNER_1 (D-27.29)\"\n\t"                                 \
            ".popsection\n\t"                                                                         \
            ".pushsection .note.niji.nonshared,\"aR\",@note\n\t"                                    \
            ".p2align 2\n\t"                                                                          \
            ".long 8\n\t"                                                                             \
            ".long 0\n\t"                                                                             \
            ".long 1\n\t"                                                                             \
            ".asciz \"libniji\"\n\t"                                                                   \
            ".p2align 2\n\t"                                                                          \
            ".popsection");
#else
#define NIJI_NONSHARED_NOTICE_FOR(name)
#endif

#endif
