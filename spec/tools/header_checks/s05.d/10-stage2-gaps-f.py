"""Slice 5 checks: stage2-gaps-f (header_checks/s05.d/, loaded after s05.py by spec/tools/header_check.py;
header_checks/README.md)."""

# The reserved scalar types <sys/types.h> shows (HEADERS-STAGE2-GAPS-F: util-linux names __int8_t .. __uint64_t), whose
# typedefs.tsv rows this story added: the exact-width and least-width integers and the remaining system-type spellings.
STAGE2F_TYPES = tuple(f"__{u}int{w}_t" for u in ("", "u") for w in (8, 16, 32, 64)) + tuple(
    f"__{u}int_least{w}_t" for u in ("", "u") for w in (8, 16, 32, 64)) + (
    "__intptr_t", "__loff_t", "__rlim_t", "__rlim64_t", "__timer_t", "__sig_atomic_t", "__suseconds64_t")


def units(ctx):
    """No spec clause (reserved names; black-box probe of the reference): each STAGE2F_TYPES name exactly where
    visibility.tsv shows it in <sys/types.h>, with its typedefs.tsv size, alignment and C++ spelling (which enters
    linkage names), and free for the program elsewhere. Defects: a missing typedef (util-linux), a width or
    signedness slip (`__int8_t` as plain char in C++ mangles differently), a name shown where the reference leaves it
    to the program."""
    return ctx.exact("sys/types.h", STAGE2F_TYPES)
