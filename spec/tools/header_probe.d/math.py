"""header_probe.py candidates: math (loaded by spec/tools/header_probe.py, header_probe.d/README.md)."""


def math_names():
    """The identifiers of the math group of headers (slice 11), which no other table spells: the types and
    macros of C 7.12 (<math.h>: float_t, double_t, HUGE_VAL*, SNAN*, FP_FAST_*), of the TS 18661 and C2X extended
    suffixes (F32 ... F64X), the POSIX XSI constants (M_E ... M_SQRT1_2), the SVID error names and the SVID
    <values.h> constants, C 7.3 (<complex.h>: _Complex_I, CMPLX*), C 7.6 (<fenv.h>) and the x87 control-word names
    of <fpu_control.h>. Only names: their visibility, types and values are probed (ruling #6047 for the pattern)."""
    suffixes = ["", "f", "l", "f32", "f64", "f128", "f32x", "f64x"]
    out = ["isgreaterequal", "isless", "islessequal", "islessgreater", "float_t", "double_t", "MAXFLOAT", "HUGE", "X_TLOSS", "DOMAIN", "SING", "OVERFLOW", "UNDERFLOW", "TLOSS",
           "PLOSS", "exception", "matherr", "_LIB_VERSION", "_IEEE_", "_SVID_", "_XOPEN_", "_POSIX_", "_ISOC_"]
    out += [f"HUGE_VAL{s.upper()}" for s in suffixes] + [f"SNAN{s.upper()}" for s in suffixes]
    out += ["FP_FAST_FMA", "FP_FAST_FMAF", "FP_FAST_FMAL"]
    for op in ("ADD", "SUB", "MUL", "DIV", "FMA", "SQRT"):
        out += [f"FP_FAST_{p}{op}{s}" for p in ("F", "D") for s in ("", "L") if not (p == "D" and s == "")]
    for c in ("E", "LOG2E", "LOG10E", "LN2", "LN10", "PI", "PI_2", "PI_4", "1_PI", "2_PI", "2_SQRTPI", "SQRT2", "SQRT1_2"):
        out += [f"M_{c}{s}" for s in suffixes]
    out += ["complex", "imaginary", "_Complex_I", "_Imaginary_I", "I"] + [f"CMPLX{s.upper()}" for s in suffixes]
    out += ["fenv_t", "fexcept_t", "femode_t", "FE_DFL_ENV", "FE_NOMASK_ENV", "FE_DFL_MODE", "FE_SNANS_ALWAYS_SIGNAL",
            "FE_ENABLE", "FE_DISABLE"]
    out += ["_FPU_MASK_IM", "_FPU_MASK_DM", "_FPU_MASK_ZM", "_FPU_MASK_OM", "_FPU_MASK_UM", "_FPU_MASK_PM",
            "_FPU_EXTENDED", "_FPU_DOUBLE", "_FPU_SINGLE", "_FPU_RC_NEAREST", "_FPU_RC_DOWN", "_FPU_RC_UP",
            "_FPU_RC_ZERO", "_FPU_RESERVED", "_FPU_DEFAULT", "_FPU_IEEE", "_FPU_GETCW", "_FPU_SETCW", "fpu_control_t",
            "__fpu_control"]
    out += ["BITSPERBYTE", "BITS", "MAXSHORT", "MAXINT", "MAXLONG", "MINSHORT", "MININT", "MINLONG", "HIBITS", "HIBITL",
            "HIBITI", "DMAXEXP", "FMAXEXP", "DMINEXP", "FMINEXP", "DSIGNIF", "FSIGNIF", "DMAXPOWTWO", "FMAXPOWTWO",
            "_DEXPLEN", "_FEXPLEN", "_HIDDENBIT", "LN_MAXDOUBLE", "LN_MINDOUBLE", "MAXDOUBLE", "MINDOUBLE", "MINFLOAT"]
    return out


def candidates():
    """(ordinary names, tag names) this family adds to the probe's candidates."""
    return math_names(), ()
