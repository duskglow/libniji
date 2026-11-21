/* libresolv through a distribution library (issue #5): linked with `-lgssapi_krb5 -lkrb5` against the
   build system's shared Kerberos libraries (copied into the root by tools/deb/root-system.sh), whose
   libkrb5.so.3 needs libresolv.so.2 and binds ns_initparse and ns_parserr at GLIBC_2.9. The root's
   loader must load libresolv.so.2 and resolve every symbol (the run sets LD_BIND_NOW) before main.
   The calls only show that both libraries are live: no krb5 or GSS-API header is needed, so the two
   functions are declared here by their documented C signatures (MIT Kerberos API reference;
   RFC 2744 section 5.27). The return codes are printed, not judged: the root has no krb5.conf. */
#include <stddef.h>
#include <stdio.h>

typedef struct _krb5_context *krb5_context;
typedef int krb5_error_code;
krb5_error_code krb5_init_context(krb5_context *context);
void krb5_free_context(krb5_context context);

typedef unsigned int OM_uint32;
typedef struct {
    size_t length;
    void *value;
} gss_buffer_desc;
OM_uint32 gss_release_buffer(OM_uint32 *minor_status, gss_buffer_desc *buffer);

int main(void)
{
    krb5_context ctx = NULL;
    krb5_error_code rc = krb5_init_context(&ctx);
    if (rc == 0)
        krb5_free_context(ctx);
    OM_uint32 minor = 0;
    gss_buffer_desc buf = { 0, NULL };
    OM_uint32 major = gss_release_buffer(&minor, &buf);
    printf("krb5: started; init_context %s, gss_release_buffer %u\n", rc == 0 ? "ok" : "error", major);
    return 0;
}
