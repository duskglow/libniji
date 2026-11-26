@@owns __bswap_16 __bswap_32 __bswap_64
/* __bswap_16, __bswap_32, __bswap_64: the bytes of x in reverse order, as static inline functions of the 16-, 32- and
   64-bit unsigned types (__uint16_t, __uint32_t, __uint64_t), with no library symbol (black-box probe of their type
   and linkage; R-H7: eudev). Each header that shows them carries this definition; the first one included defines
   them. */
@@if __bswap_16
# ifndef __NIJI_D_bswap
#  define __NIJI_D_bswap 1
static __inline unsigned short int
__bswap_16 (unsigned short int __x)
{
  return __builtin_bswap16 (__x);
}

static __inline unsigned int
__bswap_32 (unsigned int __x)
{
  return __builtin_bswap32 (__x);
}

static __inline unsigned long int
__bswap_64 (unsigned long int __x)
{
  return __builtin_bswap64 (__x);
}
# endif
#endif
