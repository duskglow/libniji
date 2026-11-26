"""header_probe.py candidates: stage2-gaps-b (loaded by spec/tools/header_probe.py, header_probe.d/README.md)."""


# The ELF type names of the System V gABI and the GNU extensions (symbol versioning, compressed sections, RELR,
# move and library entries, auxiliary vector), each in its 32- and 64-bit form (HEADERS-STAGE2-GAPS-B).
ELF_TYPES = ("Half", "Word", "Sword", "Xword", "Sxword", "Addr", "Off", "Section", "Versym", "Ehdr", "Shdr", "Chdr",
             "Sym", "Syminfo", "Rel", "Rela", "Relr", "Phdr", "Dyn", "Verdef", "Verdaux", "Verneed", "Vernaux",
             "auxv_t", "Nhdr", "Move", "Lib", "Conflict")


def stage2_gaps_b_names():
    """Names that package builds against libniji's headers failed on (HEADERS-STAGE2-GAPS-B, Yohane stage 2): apt's
    `__RES` (resolver interface version), gprofng's `RTLD_DI_SERINFOSIZE` with the other dlinfo(3) requests and the
    dladdr1(3) flags, krb5's `IN6ADDR_ANY_INIT` and `IN6ADDR_LOOPBACK_INIT` (RFC 3493 section 3.8), and babeltrace's
    `Elf32_Chdr`/`Elf64_Sword` with every 32/64-bit ELF type name (ELF_TYPES). A candidate is only a name to test."""
    out = ["__RES", "IN6ADDR_ANY_INIT", "IN6ADDR_LOOPBACK_INIT", "RTLD_DL_SYMENT", "RTLD_DL_LINKMAP"]
    out += [f"RTLD_DI_{r}" for r in ("LMID", "LINKMAP", "CONFIGADDR", "SERINFO", "SERINFOSIZE", "ORIGIN", "PROFILENAME",
                                     "PROFILEOUT", "TLS_MODID", "TLS_DATA", "PHDR", "MAX")]
    out += [f"Elf{w}_{t}" for w in (32, 64) for t in ELF_TYPES]
    # netinet/udp.h's socket options and encapsulation types (udp(7) and the kernel UAPI linux/udp.h's macro names),
    # and elf.h's ELFMAG, the string beside ELFMAG0..ELFMAG3 and SELFMAG (chrpath).
    out += ["SOL_UDP", "TCP_ENCAP_ESPINTCP", "ELFMAG"]
    out += [f"UDP_{n}" for n in ("CORK", "ENCAP", "NO_CHECK6_TX", "NO_CHECK6_RX", "SEGMENT", "GRO")]
    out += [f"UDP_ENCAP_{n}" for n in ("ESPINUDP_NON_IKE", "ESPINUDP", "L2TPINUDP", "GTP0", "GTP1U", "RXRPC")]
    # elf.h's r_info, st_info, st_other and m_info accessors (gABI "Relocation Entries", "Symbol Table"; the move
    # entries of the GNU toolchain), each width and the unsized spelling (gnu-efi).
    out += [f"ELF{w}_{m}" for w in ("32", "64", "") for m in ("R_SYM", "R_TYPE", "R_INFO", "ST_BIND", "ST_TYPE",
                                                              "ST_INFO", "ST_VISIBILITY", "M_SYM", "M_SIZE", "M_INFO")]
    # resolv.h's base64 helpers, the unprefixed names (resolver(3) family: the exported __b64_ntop and __b64_pton).
    out += ["b64_ntop", "b64_pton"]
    return out


def candidates():
    """(ordinary names, tag names) this family adds to the probe's candidates."""
    return stage2_gaps_b_names(), ()
