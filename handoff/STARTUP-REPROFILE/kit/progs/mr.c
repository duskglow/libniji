/* RELEASE-SNAPSHOT-1121A: an empty main that needs libresolv.so.2 (references one of its own functions). rstart-1's
   form `return ns_initparse == 0;` was folded by the compiler (a function's address is never null), so --as-needed
   dropped libresolv.so.2; the address now goes through a volatile, which keeps the reference (a GLOB_DAT relocation). */
extern int ns_initparse(const unsigned char *, int, void *);
int main(void) { int (*volatile p)(const unsigned char *, int, void *) = ns_initparse; return p == 0; }
