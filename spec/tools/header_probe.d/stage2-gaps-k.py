"""header_probe.py candidates: stage2-gaps-k (loaded by spec/tools/header_probe.py, header_probe.d/README.md)."""


def stage2_gaps_k_names():
    """(names, tags) of HEADERS-STAGE2-GAPS-K (Yohane stage 2, llvm-19's sanitizer_platform_limits_posix.cpp):
    <netipx/ipx.h>'s protocol-private ioctl requests (the kernel UAPI <linux/ipx.h> of the kernels that had it named
    them) and the member alias sipx_special. The probe settles which of these the reference defines, where and in
    which configurations."""
    return ["SIOCAIPXITFCRT", "SIOCAIPXPRISLT", "SIOCIPXCFGDATA", "SIOCIPXNCPCONN", "sipx_special"], []


def candidates():
    return stage2_gaps_k_names()
