/* Class adapter, floating ABI (OD-C4): the exports whose parameters or results are `long double`
 * (x87, `_Float64x`) or `_Float128`. Rust cannot name those types, so each export is one
 * statement that converts between the C type and a plain bit struct (through a union compound
 * literal) and calls the Rust implementation (`niji_*_bits`, crates/boundary/src/posix/
 * s08_numeric.rs), which works on the bits. All logic stays in Rust.
 *
 * Names are the exported glibc names under NIJI_EXPORT, and `niji_fwd_<name>` otherwise
 * (test builds never interpose on the host libc, but the forwarders still compile). Locale and
 * grouping arguments are ignored: every locale is the C locale until phase 7 (D-6.7). */
#include <stddef.h>
#include <stdint.h>
#include "niji_csys.h"

/* The bit structs the Rust side takes and returns, and its entry points. */
#ifdef NIJI_EXPORT
#define NIJI_FWD(name) name
#else
#define NIJI_FWD(name) niji_fwd_##name
#endif
struct niji_ld_bits { uint64_t mant; uint16_t se; uint16_t pad[3]; };
struct niji_f128_bits { uint64_t lo; uint64_t hi; };
union niji_ld { long double value; struct niji_ld_bits bits; };
union niji_f128 { __float128 value; struct niji_f128_bits bits; };
_Static_assert(sizeof(long double) == sizeof(struct niji_ld_bits), "long double is 16 bytes");
_Static_assert(sizeof(__float128) == sizeof(struct niji_f128_bits), "_Float128 is 16 bytes");
struct niji_ld_bits niji_strtold_bits(const char *s, char **e);
struct niji_ld_bits niji_wcstold_bits(const wchar_t *s, wchar_t **e);
struct niji_ld_bits niji_strtold_internal_bits(const char *s, char **e, int group);
struct niji_ld_bits niji_wcstold_internal_bits(const wchar_t *s, wchar_t **e, int group);
struct niji_f128_bits niji_strtof128_bits(const char *s, char **e);
struct niji_f128_bits niji_wcstof128_bits(const wchar_t *s, wchar_t **e);
int niji_strfroml_bits(char *buf, size_t n, const char *fmt, struct niji_ld_bits x);
int niji_strfromf128_bits(char *buf, size_t n, const char *fmt, struct niji_f128_bits x);
char *niji_qecvt_bits(struct niji_ld_bits x, int ndigit, int *decpt, int *sign);
char *niji_qfcvt_bits(struct niji_ld_bits x, int ndigit, int *decpt, int *sign);
int niji_qecvt_r_bits(struct niji_ld_bits x, int ndigit, int *decpt, int *sign, char *buf, size_t len);
int niji_qfcvt_r_bits(struct niji_ld_bits x, int ndigit, int *decpt, int *sign, char *buf, size_t len);
char *niji_qgcvt_bits(struct niji_ld_bits x, int ndigit, char *buf);


#define LD(expr) (((union niji_ld){ .bits = (expr) }).value)
#define LD_BITS(x) (((union niji_ld){ .value = (x) }).bits)
#define F128(expr) (((union niji_f128){ .bits = (expr) }).value)
#define F128_BITS(x) (((union niji_f128){ .value = (x) }).bits)
#define UNUSED __attribute__((unused))

/* Text to long double (§8.2, §8.3). */
long double NIJI_FWD(strtold)(const char *s, char **e) { return LD(niji_strtold_bits(s, e)); }
long double NIJI_FWD(strtold_l)(const char *s, char **e, UNUSED void *loc) { return LD(niji_strtold_bits(s, e)); }
long double NIJI_FWD(__strtold_l)(const char *s, char **e, UNUSED void *loc) { return LD(niji_strtold_bits(s, e)); }
long double NIJI_FWD(__strtold_internal)(const char *s, char **e, int group) { return LD(niji_strtold_internal_bits(s, e, group)); }
long double NIJI_FWD(strtof64x)(const char *s, char **e) { return LD(niji_strtold_bits(s, e)); }
long double NIJI_FWD(strtof64x_l)(const char *s, char **e, UNUSED void *loc) { return LD(niji_strtold_bits(s, e)); }
long double NIJI_FWD(wcstold)(const wchar_t *s, wchar_t **e) { return LD(niji_wcstold_bits(s, e)); }
long double NIJI_FWD(wcstold_l)(const wchar_t *s, wchar_t **e, UNUSED void *loc) { return LD(niji_wcstold_bits(s, e)); }
long double NIJI_FWD(__wcstold_l)(const wchar_t *s, wchar_t **e, UNUSED void *loc) { return LD(niji_wcstold_bits(s, e)); }
long double NIJI_FWD(__wcstold_internal)(const wchar_t *s, wchar_t **e, int group) { return LD(niji_wcstold_internal_bits(s, e, group)); }
long double NIJI_FWD(wcstof64x)(const wchar_t *s, wchar_t **e) { return LD(niji_wcstold_bits(s, e)); }
long double NIJI_FWD(wcstof64x_l)(const wchar_t *s, wchar_t **e, UNUSED void *loc) { return LD(niji_wcstold_bits(s, e)); }

/* Text to _Float128. */
__float128 NIJI_FWD(strtof128)(const char *s, char **e) { return F128(niji_strtof128_bits(s, e)); }
__float128 NIJI_FWD(strtof128_l)(const char *s, char **e, UNUSED void *loc) { return F128(niji_strtof128_bits(s, e)); }
__float128 NIJI_FWD(__strtof128_internal)(const char *s, char **e, UNUSED int group) { return F128(niji_strtof128_bits(s, e)); }
__float128 NIJI_FWD(wcstof128)(const wchar_t *s, wchar_t **e) { return F128(niji_wcstof128_bits(s, e)); }
__float128 NIJI_FWD(wcstof128_l)(const wchar_t *s, wchar_t **e, UNUSED void *loc) { return F128(niji_wcstof128_bits(s, e)); }
__float128 NIJI_FWD(__wcstof128_internal)(const wchar_t *s, wchar_t **e, UNUSED int group) { return F128(niji_wcstof128_bits(s, e)); }

/* Floating to text (§8.4). */
int NIJI_FWD(strfroml)(char *buf, size_t n, const char *fmt, long double x) { return niji_strfroml_bits(buf, n, fmt, LD_BITS(x)); }
int NIJI_FWD(strfromf64x)(char *buf, size_t n, const char *fmt, long double x) { return niji_strfroml_bits(buf, n, fmt, LD_BITS(x)); }
int NIJI_FWD(strfromf128)(char *buf, size_t n, const char *fmt, __float128 x) { return niji_strfromf128_bits(buf, n, fmt, F128_BITS(x)); }
char *NIJI_FWD(qecvt)(long double x, int ndigit, int *decpt, int *sign) { return niji_qecvt_bits(LD_BITS(x), ndigit, decpt, sign); }
char *NIJI_FWD(qfcvt)(long double x, int ndigit, int *decpt, int *sign) { return niji_qfcvt_bits(LD_BITS(x), ndigit, decpt, sign); }
int NIJI_FWD(qecvt_r)(long double x, int ndigit, int *decpt, int *sign, char *buf, size_t len) { return niji_qecvt_r_bits(LD_BITS(x), ndigit, decpt, sign, buf, len); }
int NIJI_FWD(qfcvt_r)(long double x, int ndigit, int *decpt, int *sign, char *buf, size_t len) { return niji_qfcvt_r_bits(LD_BITS(x), ndigit, decpt, sign, buf, len); }
char *NIJI_FWD(qgcvt)(long double x, int ndigit, char *buf) { return niji_qgcvt_bits(LD_BITS(x), ndigit, buf); }
