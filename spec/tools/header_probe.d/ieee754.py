"""header_probe.py candidates: ieee754 (loaded by spec/tools/header_probe.py, header_probe.d/README.md)."""


def candidates():
    """(ordinary names, tag names) this family adds to the probe's candidates."""
    # The tags of <ieee754.h> and the SVID `struct exception` of <math.h>.
    return (), ["exception", "ieee754_float", "ieee754_double", "ieee754_long_double", "ieee854_long_double"]
