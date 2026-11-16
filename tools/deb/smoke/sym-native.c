/* tools/deb/check-symbols.sh sample 3: a program that uses libniji's own interface
   (niji_heap_alloc@NIJI_NATIVE_1, spec/abi/native.tsv) besides the C library's: dpkg-shlibdeps
   must add niji-native-abi (>= 1), the virtual package only libniji's libc6 provides. The
   prototype is declared here so the sample needs no installed header. Built, never run. */
#include <stddef.h>
#include <stdio.h>

void *niji_heap_alloc(size_t size, size_t align);

int main(void)
{
    void *p = niji_heap_alloc(16, 16);
    printf("sample 3: %d\n", p != NULL);
    return 0;
}
