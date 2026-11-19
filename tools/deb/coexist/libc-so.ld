/* GNU ld script: libniji's C library (niji6-dev). Paths are inside the sysroot
   /usr/lib/niji; link with --sysroot=/usr/lib/niji (niji-gcc does). */
OUTPUT_FORMAT(elf64-x86-64)
GROUP ( /lib/x86_64-linux-gnu/libc.so.6 /usr/lib/x86_64-linux-gnu/libc_nonshared.a  AS_NEEDED ( /lib64/ld-linux-x86-64.so.2 ) )
