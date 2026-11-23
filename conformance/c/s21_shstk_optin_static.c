/* Spec §21.1 Guarantees and §1.8 (tunables); D-21.6, D-22.29: s21_shstk_optin's start and thread
 * rows on a static image (the start decision in the static start: the one image, no loader). */
#define S21SK_STATIC 1
#include "s21_shstk_optin.c"
