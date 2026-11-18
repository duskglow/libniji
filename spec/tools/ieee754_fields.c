/* P6 for ieee754.h (slice 11): the bit-fields of the three unions select the fields the formats define
   (IEEE 754-2019 3.4 for binary32 and binary64; the x87 extended format of the Intel SDM vol. 1, 8.2.2: 64-bit
   significand with the integer bit on top, 15-bit exponent, sign above it). Each assertion builds a value from
   one field and compares it with the number that field means, so a field one bit off is a different number.
   The expected values are written from the formats, not from the tables.
   Compiled by header_check.py in the gnu configuration. */
#include <ieee754.h>

/* binary32: sign = bit 31, exponent = bits 30..23 (bias 127), mantissa = bits 22..0. */
_Static_assert (((union ieee754_float) { .ieee = { .exponent = 127 } }).f == 1.0f, "binary32 exponent field");
_Static_assert (((union ieee754_float) { .ieee = { .exponent = 128 } }).f == 2.0f, "binary32 exponent field");
_Static_assert (((union ieee754_float) { .ieee = { .negative = 1, .exponent = 127 } }).f == -1.0f, "binary32 sign");
_Static_assert (((union ieee754_float) { .ieee = { .exponent = 127, .mantissa = 0x400000 } }).f == 1.5f, "binary32 top mantissa bit");
_Static_assert (((union ieee754_float) { .ieee = { .exponent = 127, .mantissa = 1 } }).f == 0x1.000002p0f, "binary32 low mantissa bit");
_Static_assert (((union ieee754_float) { .ieee = { .mantissa = 1 } }).f == 0x1p-149f, "binary32 smallest subnormal");
_Static_assert (__builtin_isinf (((union ieee754_float) { .ieee = { .exponent = 255 } }).f), "binary32 infinity");
/* ieee_nan: the quiet bit is the top mantissa bit (bit 22), so exponent 255 plus the quiet bit is a NaN. */
_Static_assert (__builtin_isnan (((union ieee754_float) { .ieee_nan = { .exponent = 255, .quiet_nan = 1 } }).f), "binary32 quiet NaN");
_Static_assert (__builtin_isinf (((union ieee754_float) { .ieee_nan = { .exponent = 255 } }).f), "binary32 ieee_nan exponent");

/* binary64: sign = bit 63, exponent = bits 62..52 (bias 1023), mantissa = bits 51..0 (mantissa0: 51..32, mantissa1: 31..0). */
_Static_assert (((union ieee754_double) { .ieee = { .exponent = 1023 } }).d == 1.0, "binary64 exponent field");
_Static_assert (((union ieee754_double) { .ieee = { .exponent = 1024 } }).d == 2.0, "binary64 exponent field");
_Static_assert (((union ieee754_double) { .ieee = { .negative = 1, .exponent = 1023 } }).d == -1.0, "binary64 sign");
_Static_assert (((union ieee754_double) { .ieee = { .exponent = 1023, .mantissa0 = 0x80000 } }).d == 1.5, "binary64 top mantissa bit");
_Static_assert (((union ieee754_double) { .ieee = { .exponent = 1023, .mantissa0 = 1 } }).d == 0x1.0000100000000p0, "binary64 mantissa0 low bit");
_Static_assert (((union ieee754_double) { .ieee = { .exponent = 1023, .mantissa1 = 0x80000000u } }).d == 0x1.0000080000000p0, "binary64 mantissa1 top bit");
_Static_assert (((union ieee754_double) { .ieee = { .mantissa1 = 1 } }).d == 0x1p-1074, "binary64 smallest subnormal");
_Static_assert (__builtin_isnan (((union ieee754_double) { .ieee_nan = { .exponent = 2047, .quiet_nan = 1 } }).d), "binary64 quiet NaN");
_Static_assert (__builtin_isinf (((union ieee754_double) { .ieee_nan = { .exponent = 2047 } }).d), "binary64 ieee_nan exponent");

/* x87 extended: significand = bits 63..0 (mantissa0: 63..32 with the integer bit on top, mantissa1: 31..0),
   exponent = bits 78..64 (bias 16383), sign = bit 79. */
_Static_assert (((union ieee854_long_double) { .ieee = { .exponent = 16383, .mantissa0 = 0x80000000u } }).d == 1.0L, "x87 exponent field and integer bit");
_Static_assert (((union ieee854_long_double) { .ieee = { .exponent = 16384, .mantissa0 = 0x80000000u } }).d == 2.0L, "x87 exponent field");
_Static_assert (((union ieee854_long_double) { .ieee = { .negative = 1, .exponent = 16383, .mantissa0 = 0x80000000u } }).d == -1.0L, "x87 sign");
_Static_assert (((union ieee854_long_double) { .ieee = { .exponent = 16383, .mantissa0 = 0xc0000000u } }).d == 1.5L, "x87 second significand bit");
_Static_assert (((union ieee854_long_double) { .ieee = { .exponent = 16383, .mantissa0 = 0x80000001u } }).d == 0x1.0000000200000000p0L, "x87 mantissa0 low bit");
_Static_assert (((union ieee854_long_double) { .ieee = { .exponent = 16383, .mantissa0 = 0x80000000u, .mantissa1 = 1 } }).d == 0x1.0000000000000002p0L, "x87 mantissa1 low bit");
/* ieee_nan: one = the integer bit (63), quiet_nan = the bit below it (62), mantissa0 = bits 61..32. */
_Static_assert (__builtin_isnan (((union ieee854_long_double) { .ieee_nan = { .exponent = 32767, .one = 1, .quiet_nan = 1 } }).d), "x87 quiet NaN");
_Static_assert (__builtin_isinf (((union ieee854_long_double) { .ieee_nan = { .exponent = 32767, .one = 1 } }).d), "x87 infinity");

int niji_ieee754_fields_tu;
