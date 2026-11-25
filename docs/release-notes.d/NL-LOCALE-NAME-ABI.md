---
section: changes
---
- **`_NL_LOCALE_NAME`, `static_assert` and `_STDIO_H`:** `<langinfo.h>` defines `_NL_LOCALE_NAME (category)`, whose
  `nl_langinfo` and `nl_langinfo_l` answer is the name of that category's locale, `<assert.h>` defines C11's
  `static_assert`, and `<stdio.h>` defines `_STDIO_H`. perl 5.40 chooses its interpreter structure's layout by
  whether `_NL_LOCALE_NAME` is defined, so a Perl XS module built against libniji's headers now has the handshake key
  of a perl built for the reference system and loads into it.
