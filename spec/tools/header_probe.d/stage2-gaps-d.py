"""header_probe.py candidates: stage2-gaps-d (loaded by spec/tools/header_probe.py, header_probe.d/README.md)."""


def candidates():
    """(ordinary names, tag names) this family adds to the probe's candidates."""
    # HEADERS-STAGE2-GAPS-D: perl's Configure (d_ip_mreq_source); ip(7) IP_ADD_SOURCE_MEMBERSHIP
    return (), ["ip_mreq_source"]
