@@owns __CORRECT_ISO_CPP_WCHAR_H_PROTO
@@if wcschr
# ifdef __cplusplus
/* Tells the C++ library's <cwchar> that these overloads are declared here, so it declares none of its own in std
   (R-H2, D-X.169). */
#  define __CORRECT_ISO_CPP_WCHAR_H_PROTO 1
extern "C++" {
extern const wchar_t *wcschr (const wchar_t *, wchar_t) __NIJI_NOTHROW __asm__ ("wcschr") @@attrs(wcschr);
extern const wchar_t *wcsrchr (const wchar_t *, wchar_t) __NIJI_NOTHROW __asm__ ("wcsrchr") @@attrs(wcsrchr);
extern const wchar_t *wcspbrk (const wchar_t *, const wchar_t *) __NIJI_NOTHROW __asm__ ("wcspbrk") @@attrs(wcspbrk);
extern const wchar_t *wcsstr (const wchar_t *, const wchar_t *) __NIJI_NOTHROW __asm__ ("wcsstr") @@attrs(wcsstr);
extern const wchar_t *wmemchr (const wchar_t *, wchar_t, size_t) __NIJI_NOTHROW __asm__ ("wmemchr") @@attrs(wmemchr);
inline wchar_t *wcschr (wchar_t *__s, wchar_t __c) __NIJI_NOTHROW { return const_cast<wchar_t *> (wcschr (static_cast<const wchar_t *> (__s), __c)); }
inline wchar_t *wcsrchr (wchar_t *__s, wchar_t __c) __NIJI_NOTHROW { return const_cast<wchar_t *> (wcsrchr (static_cast<const wchar_t *> (__s), __c)); }
inline wchar_t *wcspbrk (wchar_t *__s, const wchar_t *__a) __NIJI_NOTHROW { return const_cast<wchar_t *> (wcspbrk (static_cast<const wchar_t *> (__s), __a)); }
inline wchar_t *wcsstr (wchar_t *__s, const wchar_t *__a) __NIJI_NOTHROW { return const_cast<wchar_t *> (wcsstr (static_cast<const wchar_t *> (__s), __a)); }
inline wchar_t *wmemchr (wchar_t *__s, wchar_t __c, size_t __n) __NIJI_NOTHROW { return const_cast<wchar_t *> (wmemchr (static_cast<const wchar_t *> (__s), __c, __n)); }
}
# else
extern wchar_t *wcschr (const wchar_t *, wchar_t) __NIJI_NOTHROW @@attrs(wcschr);
extern wchar_t *wcsrchr (const wchar_t *, wchar_t) __NIJI_NOTHROW @@attrs(wcsrchr);
extern wchar_t *wcspbrk (const wchar_t *, const wchar_t *) __NIJI_NOTHROW @@attrs(wcspbrk);
extern wchar_t *wcsstr (const wchar_t *, const wchar_t *) __NIJI_NOTHROW @@attrs(wcsstr);
extern wchar_t *wmemchr (const wchar_t *, wchar_t, size_t) __NIJI_NOTHROW @@attrs(wmemchr);
# endif
#endif

