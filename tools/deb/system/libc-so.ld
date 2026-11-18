/* GNU ld script: libniji's C library (libc6-dev). libc.so.6 is the library,
   libc_nonshared.a holds the functions linked into the calling module, the loader
   provides the symbols the ABI places in it. */
OUTPUT_FORMAT(elf64-x86-64)
GROUP ( /lib/x86_64-linux-gnu/libc.so.6 /usr/lib/x86_64-linux-gnu/libc_nonshared.a  AS_NEEDED ( /lib64/ld-linux-x86-64.so.2 ) )
