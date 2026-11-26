@@if wcswcs
/* The XSI alias of wcsstr takes the same two overloads in C++. */
# ifdef __cplusplus
extern "C++" {
extern const wchar_t *wcswcs (const wchar_t *, const wchar_t *) __NIJI_NOTHROW __asm__ ("wcswcs") @@attrs(wcswcs);
inline wchar_t *wcswcs (wchar_t *__s, const wchar_t *__a) __NIJI_NOTHROW { return const_cast<wchar_t *> (wcswcs (static_cast<const wchar_t *> (__s), __a)); }
}
# else
extern wchar_t *wcswcs (const wchar_t *, const wchar_t *) __NIJI_NOTHROW @@attrs(wcswcs);
# endif
#endif

