/* Phase 0: the only function of the C layer, proving the build and link path. */
#include "niji_csys.h"

uint32_t niji_csys_abi_version(void)
{
    return NIJI_CSYS_ABI_VERSION;
}
