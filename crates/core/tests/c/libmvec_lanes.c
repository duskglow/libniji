/* libmvec_lanes.c (SYS-LIBMVEC): calls every libmvec.so.1 export through the x86-64 vector
 * function ABI (arguments and result in xmm/ymm/zmm registers; a `vvv` variant's pointer
 * parameters are vectors of pointers, split into registers of the ISA's integer width: 128 bits for
 * b and c, 256 for d, 512 for e) and compares each lane with the scalar libm function on the same
 * input. `exact`: every lane bit-identical to the scalar result (libniji). `ulps N`: every lane within
 * N units in the last place (a black-box check that this file's register layout is the reference
 * library's). Skips (and says so) the ISA classes the CPU lacks. Prints "lanes: checked N" and
 * exits 1 on any difference. No headers: everything is declared here. */
typedef double vd2 __attribute__((vector_size(16)));
typedef double vd4 __attribute__((vector_size(32)));
typedef double vd8 __attribute__((vector_size(64)));
typedef float vf4 __attribute__((vector_size(16)));
typedef float vf8 __attribute__((vector_size(32)));
typedef float vf16 __attribute__((vector_size(64)));
typedef long vp2 __attribute__((vector_size(16)));
typedef long vp4 __attribute__((vector_size(32)));
typedef long vp8 __attribute__((vector_size(64)));
int printf(const char *, ...);
int strcmp(const char *, const char *);
long strtol(const char *, char **, int);

static int exact = 1;
static long max_ulps;
static long checked, failed;

/* Lane i of a call: distinct per lane, inside every function's domain (acosh: 1 + x). */
static double input(const char *f, int i, int second) {
    double x = second ? 0.5 + 0.13 * i : 0.03 + 0.0571 * i;
    return (!second && !strcmp(f, "acosh")) ? 1.0 + x : x;
}

static long ulps_d(double a, double b) {
    union { double d; long l; } x = {a}, y = {b};
    if (x.l < 0) x.l = (long)(0x8000000000000000UL - (unsigned long)x.l);
    if (y.l < 0) y.l = (long)(0x8000000000000000UL - (unsigned long)y.l);
    return x.l > y.l ? x.l - y.l : y.l - x.l;
}
static long ulps_f(float a, float b) {
    union { float f; int i; } x = {a}, y = {b};
    long p = x.i < 0 ? (long)0x80000000L - (long)(unsigned)x.i : x.i;
    long q = y.i < 0 ? (long)0x80000000L - (long)(unsigned)y.i : y.i;
    return p > q ? p - q : q - p;
}
static void report(const char *name, int lane, long d, int same) {
    checked++;
    if (exact ? !same : d > max_ulps) {
        failed++;
        printf("DIFF %s lane %d: %ld ulps from the scalar result\n", name, lane, d);
    }
}
static void cmp_d(const char *n, int lane, double v, double s) {
    union { double d; long l; } a = {v}, b = {s};
    report(n, lane, ulps_d(v, s), a.l == b.l);
}
static void cmp_f(const char *n, int lane, float v, float s) {
    union { float f; int i; } a = {v}, b = {s};
    report(n, lane, ulps_f(v, s), a.i == b.i);
}

/* The functions: one-argument (23 per type), two-argument (3 per type), sincos. */
#define V1(X) X(acos) X(acosh) X(asin) X(asinh) X(atan) X(atanh) X(cbrt) X(cos) X(cosh) X(erf) X(erfc) \
    X(exp) X(exp10) X(exp2) X(expm1) X(log) X(log10) X(log1p) X(log2) X(sin) X(sinh) X(tan) X(tanh)
#define V2(X) X(atan2) X(hypot) X(pow)
/* ISA classes: letter, target, double lanes and vector type, float lanes and vector type. */
#define ISAS(X, F) X(b, "sse2", 2, vd2, 4, vf4, F) X(c, "avx", 4, vd4, 8, vf8, F) \
    X(d, "avx2", 4, vd4, 8, vf8, F) X(e, "avx512f", 8, vd8, 16, vf16, F)

/* The float function of double function f: pasting f to itself would repeat the name. */
#define FLT(x) x##f
#define SCALAR(f) double f(double); float FLT(f)(float);
V1(SCALAR)
#define SCALAR2(f) double f(double, double); float FLT(f)(float, float);
V2(SCALAR2)
void sincos(double, double *, double *);
void sincosf(float, float *, float *);

#define DECL1(i, t, nd, vd, nf, vf, f) vd _ZGV##i##N##nd##v_##f(vd); vf FLT(_ZGV##i##N##nf##v_##f)(vf);
#define DECL1_ALL(f) ISAS(DECL1, f)
V1(DECL1_ALL)
#define DECL2(i, t, nd, vd, nf, vf, f) vd _ZGV##i##N##nd##vv_##f(vd, vd); vf FLT(_ZGV##i##N##nf##vv_##f)(vf, vf);
#define DECL2_ALL(f) ISAS(DECL2, f)
V2(DECL2_ALL)

#define CHK1(i, t, nd, vd, nf, vf, f) \
    __attribute__((target(t), noinline)) static void chk1_##i##_##f(void) { \
        vd x; vf y; \
        for (int k = 0; k < nd; k++) x[k] = input(#f, k, 0); \
        for (int k = 0; k < nf; k++) y[k] = (float)input(#f, k, 0); \
        vd r = _ZGV##i##N##nd##v_##f(x); vf q = FLT(_ZGV##i##N##nf##v_##f)(y); \
        for (int k = 0; k < nd; k++) cmp_d("_ZGV" #i "N" #nd "v_" #f, k, r[k], f(x[k])); \
        for (int k = 0; k < nf; k++) cmp_f("_ZGV" #i "N" #nf "v_" #f "f", k, q[k], FLT(f)(y[k])); }
#define CHK1_ALL(f) ISAS(CHK1, f)
V1(CHK1_ALL)
#define CHK2(i, t, nd, vd, nf, vf, f) \
    __attribute__((target(t), noinline)) static void chk2_##i##_##f(void) { \
        vd x, x2; vf y, y2; \
        for (int k = 0; k < nd; k++) { x[k] = input(#f, k, 0); x2[k] = input(#f, k, 1); } \
        for (int k = 0; k < nf; k++) { y[k] = (float)input(#f, k, 0); y2[k] = (float)input(#f, k, 1); } \
        vd r = _ZGV##i##N##nd##vv_##f(x, x2); vf q = FLT(_ZGV##i##N##nf##vv_##f)(y, y2); \
        for (int k = 0; k < nd; k++) cmp_d("_ZGV" #i "N" #nd "vv_" #f, k, r[k], f(x[k], x2[k])); \
        for (int k = 0; k < nf; k++) cmp_f("_ZGV" #i "N" #nf "vv_" #f "f", k, q[k], FLT(f)(y[k], y2[k])); }
#define CHK2_ALL(f) ISAS(CHK2, f)
V2(CHK2_ALL)

/* sincos: the pointer vectors, as the ISA's integer registers hold them. */
void _ZGVbN2vvv_sincos(vd2, vp2, vp2);
void _ZGVcN4vvv_sincos(vd4, vp2, vp2, vp2, vp2);
void _ZGVdN4vvv_sincos(vd4, vp4, vp4);
void _ZGVeN8vvv_sincos(vd8, vp8, vp8);
void _ZGVbN4vvv_sincosf(vf4, vp2, vp2, vp2, vp2);
void _ZGVcN8vvv_sincosf(vf8, vp2, vp2, vp2, vp2, vp2, vp2, vp2, vp2);
void _ZGVdN8vvv_sincosf(vf8, vp4, vp4, vp4, vp4);
void _ZGVeN16vvv_sincosf(vf16, vp8, vp8, vp8, vp8);

/* Fill pointer vectors p[0..] (each of `per` lanes) with &a[lane]. */
#define PTRS(T, p, per, nv, a) for (int k = 0; k < (per) * (nv); k++) p[k / (per)][k % (per)] = (long)&a[k];
#define SC_CHECK(T, n, x, s, c, name, cmp) \
    for (int k = 0; k < n; k++) { T ss, cc; sincos##name(x[k], &ss, &cc); \
        cmp("sin of " #n "-lane sincos" #name, k, s[k], ss); cmp("cos of " #n "-lane sincos" #name, k, c[k], cc); }

__attribute__((target("sse2"), noinline)) static void chk_sincos_b(void) {
    vd2 x; double s[2], c[2]; vp2 ps[1], pc[1];
    for (int k = 0; k < 2; k++) x[k] = input("sincos", k, 0);
    PTRS(double, ps, 2, 1, s) PTRS(double, pc, 2, 1, c)
    _ZGVbN2vvv_sincos(x, ps[0], pc[0]);
    SC_CHECK(double, 2, x, s, c, , cmp_d)
    vf4 y; float sf[4], cf[4]; vp2 qs[2], qc[2];
    for (int k = 0; k < 4; k++) y[k] = (float)input("sincos", k, 0);
    PTRS(float, qs, 2, 2, sf) PTRS(float, qc, 2, 2, cf)
    _ZGVbN4vvv_sincosf(y, qs[0], qs[1], qc[0], qc[1]);
    SC_CHECK(float, 4, y, sf, cf, f, cmp_f)
}
__attribute__((target("avx"), noinline)) static void chk_sincos_c(void) {
    vd4 x; double s[4], c[4]; vp2 ps[2], pc[2];
    for (int k = 0; k < 4; k++) x[k] = input("sincos", k, 0);
    PTRS(double, ps, 2, 2, s) PTRS(double, pc, 2, 2, c)
    _ZGVcN4vvv_sincos(x, ps[0], ps[1], pc[0], pc[1]);
    SC_CHECK(double, 4, x, s, c, , cmp_d)
    vf8 y; float sf[8], cf[8]; vp2 qs[4], qc[4];
    for (int k = 0; k < 8; k++) y[k] = (float)input("sincos", k, 0);
    PTRS(float, qs, 2, 4, sf) PTRS(float, qc, 2, 4, cf)
    _ZGVcN8vvv_sincosf(y, qs[0], qs[1], qs[2], qs[3], qc[0], qc[1], qc[2], qc[3]);
    SC_CHECK(float, 8, y, sf, cf, f, cmp_f)
}
__attribute__((target("avx2"), noinline)) static void chk_sincos_d(void) {
    vd4 x; double s[4], c[4]; vp4 ps[1], pc[1];
    for (int k = 0; k < 4; k++) x[k] = input("sincos", k, 0);
    PTRS(double, ps, 4, 1, s) PTRS(double, pc, 4, 1, c)
    _ZGVdN4vvv_sincos(x, ps[0], pc[0]);
    SC_CHECK(double, 4, x, s, c, , cmp_d)
    vf8 y; float sf[8], cf[8]; vp4 qs[2], qc[2];
    for (int k = 0; k < 8; k++) y[k] = (float)input("sincos", k, 0);
    PTRS(float, qs, 4, 2, sf) PTRS(float, qc, 4, 2, cf)
    _ZGVdN8vvv_sincosf(y, qs[0], qs[1], qc[0], qc[1]);
    SC_CHECK(float, 8, y, sf, cf, f, cmp_f)
}
__attribute__((target("avx512f"), noinline)) static void chk_sincos_e(void) {
    vd8 x; double s[8], c[8]; vp8 ps[1], pc[1];
    for (int k = 0; k < 8; k++) x[k] = input("sincos", k, 0);
    PTRS(double, ps, 8, 1, s) PTRS(double, pc, 8, 1, c)
    _ZGVeN8vvv_sincos(x, ps[0], pc[0]);
    SC_CHECK(double, 8, x, s, c, , cmp_d)
    vf16 y; float sf[16], cf[16]; vp8 qs[2], qc[2];
    for (int k = 0; k < 16; k++) y[k] = (float)input("sincos", k, 0);
    PTRS(float, qs, 8, 2, sf) PTRS(float, qc, 8, 2, cf)
    _ZGVeN16vvv_sincosf(y, qs[0], qs[1], qc[0], qc[1]);
    SC_CHECK(float, 16, y, sf, cf, f, cmp_f)
}

#define RUN1(i, t, nd, vd, nf, vf, f) chk1_##i##_##f();
#define RUN2(i, t, nd, vd, nf, vf, f) chk2_##i##_##f();
#define ISA_RUN(i, t, nd, vd, nf, vf, F) \
    if (__builtin_cpu_supports(t)) { RUN_ONE_##i() chk_sincos_##i(); } \
    else printf("lanes: skip ISA class " #i ": the CPU has no %s\n", t);
#define R1_b(f) RUN1(b, , , , , , f)
#define R1_c(f) RUN1(c, , , , , , f)
#define R1_d(f) RUN1(d, , , , , , f)
#define R1_e(f) RUN1(e, , , , , , f)
#define R2_b(f) RUN2(b, , , , , , f)
#define R2_c(f) RUN2(c, , , , , , f)
#define R2_d(f) RUN2(d, , , , , , f)
#define R2_e(f) RUN2(e, , , , , , f)
#define RUN_ONE_b() V1(R1_b) V2(R2_b)
#define RUN_ONE_c() V1(R1_c) V2(R2_c)
#define RUN_ONE_d() V1(R1_d) V2(R2_d)
#define RUN_ONE_e() V1(R1_e) V2(R2_e)

int main(int argc, char **argv) {
    if (argc > 2 && !strcmp(argv[1], "ulps")) { exact = 0; max_ulps = strtol(argv[2], 0, 10); }
    __builtin_cpu_init();
    ISAS(ISA_RUN, )
    printf("lanes: checked %ld, %ld differ (%s)\n", checked, failed, exact ? "bit-exact" : "ulps bound");
    return failed != 0;
}
