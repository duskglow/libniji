/* Inline body when optimizing (features.h, __NIJI_INLINES): mbrlen is mbrtowc (NULL, s, n, ps) for a caller's state
   (C 7.29.6.3.1); without one it keeps its own internal state, which the library entry __mbrlen holds. */
#if __NIJI_INLINES
@@when mbrlen
__NIJI_INLINE_FN size_t mbrlen (const char *__restrict __s, size_t __n, mbstate_t *__restrict __ps) __NIJI_NOTHROW
{
  return __ps != 0 ? mbrtowc ((wchar_t *) 0, __s, __n, __ps) : __mbrlen (__s, __n, (mbstate_t *) 0);
}
#endif
#endif
