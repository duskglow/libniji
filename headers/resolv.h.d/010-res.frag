@@owns-macro res_init res_nclose res_ninit _res b64_ntop b64_pton
/* res_init, res_ninit and res_nclose name the exported __res_* functions (declared above). */
@@when _res
# define res_init __res_init
# define res_ninit __res_ninit
# define res_nclose __res_nclose
# define b64_ntop __b64_ntop
# define b64_pton __b64_pton
# define _res (*__res_state ())
#endif
