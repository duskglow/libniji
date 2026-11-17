/* The libm.so.6 compat versions old binaries bind (spec s0_6_c0_6_0; spec/behaviour/libm-compat.md).
 *
 * Binds each name's old version with `.symver` (a program linked anew cannot reach it otherwise) and
 * calls it on edge cases: zero and signed zero, a zero divisor, infinities, quiet and signalling NaNs,
 * subnormals and values near overflow. For every call it prints the result's bits, errno (cleared
 * first) and the raised exception flags, and whether all three equal the default version's. The
 * reference and libniji must print the same text (crates/core/tests/crt_products.rs).
 *
 * Build: cc -O1 -fno-builtin libm_compat_versions.c -lm
 * With -DPROBE_ALL it also binds the old versions libniji does not export (the reference's record,
 * spec/behaviour/libm-compat.glibc.txt), and prints only a summary per name and the differing cases. */
#define _GNU_SOURCE
#include <errno.h>
#include <fenv.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#define OLD(n, ret, args) ret n##_old args; __asm__(".symver " #n "_old," #n "@GLIBC_2.2.5");
#define NEW(n, v, ret, args) ret n##_new args; __asm__(".symver " #n "_new," #n "@" v);

/* The old versions libniji exports, with the default version each is compared with. */
OLD(fmod, double, (double, double)) NEW(fmod, "GLIBC_2.38", double, (double, double))
OLD(fmodf, float, (float, float)) NEW(fmodf, "GLIBC_2.38", float, (float, float))
OLD(exp, double, (double)) NEW(exp, "GLIBC_2.29", double, (double))
OLD(exp10, double, (double)) NEW(exp10, "GLIBC_2.39", double, (double))
OLD(pow, double, (double, double)) NEW(pow, "GLIBC_2.29", double, (double, double))
OLD(hypot, double, (double, double)) NEW(hypot, "GLIBC_2.35", double, (double, double))
OLD(hypotf, float, (float, float)) NEW(hypotf, "GLIBC_2.35", float, (float, float))
OLD(log, double, (double)) NEW(log, "GLIBC_2.29", double, (double))
OLD(log2, double, (double)) NEW(log2, "GLIBC_2.29", double, (double))
#ifdef PROBE_ALL
OLD(exp2, double, (double)) NEW(exp2, "GLIBC_2.29", double, (double))
OLD(lgamma, double, (double)) NEW(lgamma, "GLIBC_2.23", double, (double))
OLD(expf, float, (float)) NEW(expf, "GLIBC_2.27", float, (float))
OLD(exp2f, float, (float)) NEW(exp2f, "GLIBC_2.27", float, (float))
OLD(exp10f, float, (float)) NEW(exp10f, "GLIBC_2.32", float, (float))
OLD(logf, float, (float)) NEW(logf, "GLIBC_2.27", float, (float))
OLD(log2f, float, (float)) NEW(log2f, "GLIBC_2.27", float, (float))
OLD(lgammaf, float, (float)) NEW(lgammaf, "GLIBC_2.23", float, (float))
OLD(powf, float, (float, float)) NEW(powf, "GLIBC_2.27", float, (float, float))
OLD(lgammal, long double, (long double)) NEW(lgammal, "GLIBC_2.23", long double, (long double))
#endif

/* Whether a library function raises "inexact" is unspecified (C17 F.10), so it is not compared (the
 * reference raises it for exp10(1) = 10 exactly, libniji does not). */
#define FLAGS (FE_INVALID | FE_DIVBYZERO | FE_OVERFLOW | FE_UNDERFLOW)
#define N 22
#define SNAN_AT 19
/* The logarithms also take a quiet and a signalling NaN with the sign bit set (indices N, N + 1):
 * their old version differs from the default in a NaN's sign (spec/behaviour/libm-compat.md). */
#define NLOG (N + 2)

struct obs {
    char bits[48];
    int err, flags, sg;
};

static double dval(int i)
{
    static const double v[N] = {0.0, -0.0, 1.0, -1.0, 0.5, 2.5, 7.5, -3.0, -2.0, 710.0, -746.0, 1e-310, -1e-310,
                                1e308, -1e308, 1e-5, 1024.0, -1075.0, NAN, 0, INFINITY, -INFINITY};
    uint64_t s = i == N ? 0xfff8000000000000ull : i == N + 1 ? 0xfff4000000000000ull : 0x7ff4000000000000ull;
    double d;
    memcpy(&d, &s, sizeof d);
    return i == SNAN_AT || i >= N ? d : v[i];
}

static float fval(int i)
{
    static const float v[N] = {0.0f, -0.0f, 1.0f, -1.0f, 0.5f, 2.5f, 7.5f, -3.0f, -2.0f, 89.0f, -104.0f, 1e-40f, -1e-40f,
                               3e38f, -3e38f, 1e-5f, 128.0f, -150.0f, NAN, 0, INFINITY, -INFINITY};
    uint32_t s = i == N ? 0xffc00000u : i == N + 1 ? 0xffa00000u : 0x7fa00000u;
    float f;
    memcpy(&f, &s, sizeof f);
    return i == SNAN_AT || i >= N ? f : v[i];
}

static int verbose, cases, diffs;

static void start(void)
{
    signgam = 7;
    feclearexcept(FLAGS);
    errno = 0;
}

static void finish(struct obs *o)
{
    o->err = errno;
    o->flags = fetestexcept(FLAGS);
    o->sg = signgam;
}

static void report(const char *name, const char *in, const struct obs *a, const struct obs *b)
{
    int same = !strcmp(a->bits, b->bits) && a->err == b->err && a->flags == b->flags && a->sg == b->sg;
    cases++;
    diffs += !same;
    if (verbose || !same)
        printf("%s@GLIBC_2.2.5(%s) = %s errno %d flags %#x signgam %d: %s\n", name, in, a->bits, a->err, a->flags,
               a->sg, same ? "same as default" : "DIFFERS from default");
    if (!same)
        printf("  default: %s errno %d flags %#x signgam %d\n", b->bits, b->err, b->flags, b->sg);
}

static void summary(const char *name)
{
    printf("%s@GLIBC_2.2.5: %d cases, %d differ from the default\n", name, cases, diffs);
    cases = diffs = 0;
}

static void dbits(struct obs *o, double r)
{
    uint64_t u;
    memcpy(&u, &r, sizeof u);
    snprintf(o->bits, sizeof o->bits, "%016llx", (unsigned long long)u);
}

static void fbits(struct obs *o, float r)
{
    uint32_t u;
    memcpy(&u, &r, sizeof u);
    snprintf(o->bits, sizeof o->bits, "%08x", u);
}

typedef double (*d2)(double, double);
typedef float (*f2)(float, float);
typedef double (*d1)(double);
typedef float (*f1)(float);

static void run_d2(const char *name, d2 old, d2 def)
{
    for (int i = 0; i < N; i++)
        for (int j = 0; j < N; j++) {
            volatile double x = dval(i), y = dval(j);
            struct obs a, b;
            char in[80];
            start(), dbits(&a, old(x, y)), finish(&a);
            start(), dbits(&b, def(x, y)), finish(&b);
            snprintf(in, sizeof in, "%a, %a", (double)x, (double)y);
            report(name, in, &a, &b);
        }
    summary(name);
}

static void run_f2(const char *name, f2 old, f2 def)
{
    for (int i = 0; i < N; i++)
        for (int j = 0; j < N; j++) {
            volatile float x = fval(i), y = fval(j);
            struct obs a, b;
            char in[80];
            start(), fbits(&a, old(x, y)), finish(&a);
            start(), fbits(&b, def(x, y)), finish(&b);
            snprintf(in, sizeof in, "%a, %a", (double)x, (double)y);
            report(name, in, &a, &b);
        }
    summary(name);
}

static void run_d1(const char *name, d1 old, d1 def, int n)
{
    for (int i = 0; i < n; i++) {
        volatile double x = dval(i);
        struct obs a, b;
        char in[40];
        start(), dbits(&a, old(x)), finish(&a);
        start(), dbits(&b, def(x)), finish(&b);
        snprintf(in, sizeof in, "%a", (double)x);
        report(name, in, &a, &b);
    }
    summary(name);
}

#ifdef PROBE_ALL
static void run_f1(const char *name, f1 old, f1 def, int n)
{
    for (int i = 0; i < n; i++) {
        volatile float x = fval(i);
        struct obs a, b;
        char in[40];
        start(), fbits(&a, old(x)), finish(&a);
        start(), fbits(&b, def(x)), finish(&b);
        snprintf(in, sizeof in, "%a", (double)x);
        report(name, in, &a, &b);
    }
    summary(name);
}

static void run_l1(const char *name)
{
    for (int i = 0; i < N; i++) {
        volatile long double x = dval(i);
        struct obs a, b;
        char in[40];
        start(), snprintf(a.bits, sizeof a.bits, "%La", lgammal_old(x)), finish(&a);
        start(), snprintf(b.bits, sizeof b.bits, "%La", lgammal_new(x)), finish(&b);
        snprintf(in, sizeof in, "%La", (long double)x);
        report(name, in, &a, &b);
    }
    summary(name);
}
#endif

int main(int argc, char **argv)
{
    verbose = argc > 1 && !strcmp(argv[1], "-v");
    run_d2("fmod", fmod_old, fmod_new);
    run_f2("fmodf", fmodf_old, fmodf_new);
    run_d1("exp", exp_old, exp_new, N);
    run_d1("exp10", exp10_old, exp10_new, N);
    run_d2("pow", pow_old, pow_new);
    run_d2("hypot", hypot_old, hypot_new);
    run_f2("hypotf", hypotf_old, hypotf_new);
    run_d1("log", log_old, log_new, NLOG);
    run_d1("log2", log2_old, log2_new, NLOG);
#ifdef PROBE_ALL
    run_d1("exp2", exp2_old, exp2_new, N);
    run_d1("lgamma", lgamma_old, lgamma_new, N);
    run_f1("expf", expf_old, expf_new, N);
    run_f1("exp2f", exp2f_old, exp2f_new, N);
    run_f1("exp10f", exp10f_old, exp10f_new, N);
    run_f1("logf", logf_old, logf_new, NLOG);
    run_f1("log2f", log2f_old, log2f_new, NLOG);
    run_f1("lgammaf", lgammaf_old, lgammaf_new, N);
    run_f2("powf", powf_old, powf_new);
    run_l1("lgammal");
#endif
    return 0;
}
