/* tools/deb/check-symbols.sh sample 3: a program that uses Liella's own interface
   (liella_heap_alloc@LIELLA_NATIVE_1, spec/abi/native.tsv) besides the C library's: dpkg-shlibdeps
   must add liella-native-abi (>= 1), the virtual package only Liella's libc6 provides. The
   prototype is declared here so the sample needs no installed header. Built, never run. */
#include <stddef.h>
#include <stdio.h>

void *liella_heap_alloc(size_t size, size_t align);

int main(void)
{
    void *p = liella_heap_alloc(16, 16);
    printf("sample 3: %d\n", p != NULL);
    return 0;
}
