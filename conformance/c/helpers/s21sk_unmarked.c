/* s21_shstk_optin: an object built without the shadow-stack property (HELPER-FLAGS
 * -fcf-protection=none), which a dlopen after the start must treat as unmarked. */
int s21sk_fn(void) { return 5; }
