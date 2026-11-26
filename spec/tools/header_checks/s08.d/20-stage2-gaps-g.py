"""Slice 8 checks: stage2-gaps-g (header_checks/s08.d/, loaded after s08.py by spec/tools/header_check.py;
header_checks/README.md)."""


def units(ctx):
    """No spec clause (reserved names; black-box probe of the reference, HEADERS-STAGE2-GAPS-G: gcc-14's libgcc
    generic-morestack.c names them): __SIGRTMIN and __SIGRTMAX defined exactly where visibility.tsv shows them in
    <signal.h> (P1 checks their values). Defects: missing (libgcc does not build), or shown where the reference leaves
    them to the program."""
    return ctx.exact("signal.h", ("__SIGRTMIN", "__SIGRTMAX"))
