/* C++: issignaling, iszero, iscanonical and iseqsig are functions, not macros (ISO/IEC TS 18661-1 macros; the C++
   library declares none of them). Their form is the reference's (black-box probe, HEADERS-STAGE2-GAPS-L, C++98 to
   C++23): issignaling and iscanonical one overload per floating type (int; _Float128 where the compiler has it as a
   C++ type), iszero one template (bool), iseqsig one template over both argument types (int, noexcept), whose
   integer arguments compare as double (C 7.12.17). */
#if __NIJI_CXX >= 1 && __NIJI_ISOC >= 2023
} /* extern "C" */
# if defined __GNUC__ && !defined __clang__ && __GNUC__ >= 13
#  define __NIJI_CXX_F128 1
# endif
inline int issignaling (float __x) { return __builtin_issignaling (__x); }
inline int issignaling (double __x) { return __builtin_issignaling (__x); }
inline int issignaling (long double __x) { return __builtin_issignaling (__x); }
inline int iscanonical (float __x) { return ((void) __x, 1); }
inline int iscanonical (double __x) { return ((void) __x, 1); }
/* The x87 type has encodings that are not canonical (pseudo-denormals, unnormals, ...): the library tests those bits. */
inline int iscanonical (long double __x) { return __iscanonicall (__x); }
# ifdef __NIJI_CXX_F128
inline int issignaling (_Float128 __x) { return __builtin_issignaling (__x); }
inline int iscanonical (_Float128 __x) { return ((void) __x, 1); }
# endif
template <class __T> inline bool iszero (__T __x) { return __x == 0; }
/* The type both arguments convert to: their usual arithmetic conversion, an integer type taken as double. Other
   types (__float128 where it is not _Float128) have none. */
template <class __T> struct __niji_eqsig_type;
template <> struct __niji_eqsig_type<float> { typedef float __type; };
template <> struct __niji_eqsig_type<double> { typedef double __type; };
template <> struct __niji_eqsig_type<long double> { typedef long double __type; };
template <> struct __niji_eqsig_type<int> { typedef double __type; };
template <> struct __niji_eqsig_type<unsigned int> { typedef double __type; };
template <> struct __niji_eqsig_type<long int> { typedef double __type; };
template <> struct __niji_eqsig_type<unsigned long int> { typedef double __type; };
template <> struct __niji_eqsig_type<long long int> { typedef double __type; };
template <> struct __niji_eqsig_type<unsigned long long int> { typedef double __type; };
# ifdef __NIJI_CXX_F128
template <> struct __niji_eqsig_type<_Float128> { typedef _Float128 __type; };
template <> struct __niji_eqsig_type<_Float32> { typedef float __type; };
template <> struct __niji_eqsig_type<_Float64> { typedef double __type; };
template <> struct __niji_eqsig_type<_Float32x> { typedef double __type; };
template <> struct __niji_eqsig_type<_Float64x> { typedef long double __type; };
# endif
/* C2X 7.12.17.7: the relational operators raise "invalid" for a quiet NaN too, as the signaling comparison needs. */
template <class __L, class __R>
inline int iseqsig (__L __l, __R __r) __NIJI_NOTHROW
{
  typedef typename __niji_eqsig_type<__typeof__ (__l + __r)>::__type __t;
  return ((__t) __l <= (__t) __r) && ((__t) __l >= (__t) __r);
}
# undef __NIJI_CXX_F128
extern "C" {
#endif
