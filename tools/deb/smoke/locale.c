/* Smoke program for the locales package (tools/deb/root-system.sh, UTIL-LOCALE-GEN): takes the
   locale the environment names and prints its name, its codeset and Monday's name. */
#include <langinfo.h>
#include <locale.h>
#include <stdio.h>

int main(void) {
    const char *name = setlocale(LC_ALL, "");
    printf("%s %s %s\n", name ? name : "(null)", nl_langinfo(CODESET), nl_langinfo(DAY_2));
    return name == NULL;
}
