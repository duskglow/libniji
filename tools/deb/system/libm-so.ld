/* GNU ld script: libniji's maths library (libc6-dev), what -lm links. libm.so.6 is the
   library. libmvec.so.1 holds the vector variants a vectorised loop calls (_ZGV*); AS_NEEDED
   records it only when an object uses one of its symbols, so a program that uses none
   gains no dependency on it. */
OUTPUT_FORMAT(elf64-x86-64)
GROUP ( /lib/x86_64-linux-gnu/libm.so.6  AS_NEEDED ( /lib/x86_64-linux-gnu/libmvec.so.1 ) )
