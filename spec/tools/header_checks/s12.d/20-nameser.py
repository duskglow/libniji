"""Slice 12 checks: nameser (header_checks/s12.d/, loaded after s12.py by spec/tools/header_check.py; header_checks/README.md)."""
from header_check import expand


def nameser_enum_units(ctx):
    """No spec clause beyond RFC 1035's field values (the type of the names by black-box probe, HEADERS-ENUM-FOLLOWUPS):
    per configuration that shows them, every enumeration `enum __ns_*` of <arpa/nameser.h> is declared with each of its
    enumerators.tsv members at its value, a member converts to its enumeration without a cast (C++: an int or a
    structure in its place fails), the typedefs `ns_type`, `ns_class`, `ns_sect` name those enumerations, and a DNS
    style switch over `ns_class` compiles under -Wall -Werror. Defects it catches: an enumeration left opaque, an
    enumeration of a reserved tag dropped with all its public members (`ns_r_servfail`), a wrong value."""
    tags = sorted(r["name"] for r in ctx.vis if r["header"] == "arpa/nameser.h" and r["kind"] == "enum" and r["name"].startswith("__ns_"))
    members = {}
    for r in ctx.rows("spec/abi/enumerators.tsv"):
        if r["tag"].startswith("enum ") and r["tag"][5:] in tags:
            members.setdefault(r["tag"][5:], []).append((r["name"], r["value"]))
    shown = {r["name"]: r["configs"] for r in ctx.vis if r["header"] == "arpa/nameser.h" and r["kind"] in ("enumerator", "enum", "typedef")}
    if len(tags) < 8 or set(members) != set(tags) or not {"ns_type", "ns_class", "ns_sect"} <= set(shown):
        return [(ctx.ids[0], "enum arpa/nameser.h", "#error the tables do not show the eight __ns_* enumerations and their typedefs\n")]
    lang = {c["id"]: c["language"] for c in ctx.cfgs}
    res = []
    for cid in ctx.ids:
        if cid not in expand(shown["ns_type"], ctx.ids):
            continue
        body = ["#include <arpa/nameser.h>", "ns_type niji_t = ns_t_a;", "ns_sect niji_s = ns_s_an;",
                "int niji_ns_f (ns_class c) { switch (c) { case ns_c_in: return 1; case ns_c_chaos: return 3; default: return 0; } }"]
        for tag in tags:
            for i, (n, v) in enumerate(members[tag]):
                if cid not in expand(shown.get(n, ""), ctx.ids):
                    continue
                body += [f"enum {tag} niji_{tag}_{i} = {n};", f"typedef char niji_v{tag}_{i}[({n}) == ({v}) ? 1 : -1];"]
            if lang[cid] == "c++":
                body.append(f"typedef char niji_k{tag}[__is_enum (enum {tag}) ? 1 : -1];")
        res.append((cid, "enum arpa/nameser.h", "\n".join(body) + "\n"))
    return res


NAMESER_ACCESSORS = ("ns_msg_id", "ns_msg_base", "ns_msg_end", "ns_msg_size", "ns_msg_count", "ns_rr_name", "ns_rr_type",
                     "ns_rr_class", "ns_rr_ttl", "ns_rr_rdlen", "ns_rr_rdata", "NS_GET16", "NS_GET32", "NS_PUT16",
                     "NS_PUT32", "GETSHORT", "GETLONG", "PUTSHORT", "PUTLONG")
# Each accessor's type in the reference (black-box form probe, HEADERS-STAGE2-GAPS-E); every one is a prvalue there.
NAMESER_FORMS = (("ns_msg_id (niji_h)", "int"), ("ns_msg_base (niji_h)", "const unsigned char *"),
                 ("ns_msg_end (niji_h)", "const unsigned char *"), ("ns_msg_size (niji_h)", "long"),
                 ("ns_msg_count (niji_h, ns_s_an)", "int"), ("ns_rr_name (niji_rr)", "const char * (C: char *)"),
                 ("ns_rr_type (niji_rr)", "ns_type"), ("ns_rr_class (niji_rr)", "ns_class"),
                 ("ns_rr_ttl (niji_rr)", "unsigned int"), ("ns_rr_rdlen (niji_rr)", "int"),
                 ("ns_rr_rdata (niji_rr)", "const unsigned char *"))
# C++14 on: the values, evaluated at compile time (each check returns its own number when it fails). The byte-order
# macros read and write big-endian (RFC 1035 section 2.3.2), keep the low 16 or 32 bits and advance the pointer.
NAMESER_VALUES = """constexpr int niji_ns_values () {
  ns_msg h {}; ns_rr rr {};
  unsigned char b[8] = {0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0}, o[8] = {};
  h._msg = b; h._eom = b + 6; h._id = 0x1234; h._counts[0] = 11; h._counts[2] = 33;
  if (ns_msg_id (h) != 0x1234 || ns_msg_base (h) != b || ns_msg_end (h) != b + 6 || ns_msg_size (h) != 6) return 1;
  if (ns_msg_count (h, ns_s_qd) != 11 || ns_msg_count (h, ns_s_ns) != 33 || ns_msg_count (h, 1 ? 2 : 0) != 33) return 2;
  rr.type = 18; rr.rr_class = 1; rr.ttl = 300; rr.rdlength = 7; rr.rdata = b + 3;
  if (ns_rr_type (rr) != ns_t_afsdb || ns_rr_class (rr) != ns_c_in || ns_rr_ttl (rr) != 300 || ns_rr_rdlen (rr) != 7
      || ns_rr_rdata (rr) != b + 3) return 3;
  if (ns_rr_name (rr)[0] != '.' || ns_rr_name (rr)[1] != 0) return 4;
  rr.name[0] = 'a';
  if (ns_rr_name (rr) != rr.name) return 5;
  const unsigned char *cp = b; unsigned int s = 0; unsigned long l = 0;
  NS_GET16 (s, cp); NS_GET32 (l, cp);
  if (s != 0x1234 || l != 0x56789abcUL || cp != b + 6) return 6;
  unsigned char *wp = o;
  NS_PUT16 (0xa1b2c3, wp); NS_PUT32 (0x1122334455ULL, wp);
  if (o[0] != 0xb2 || o[1] != 0xc3 || o[2] != 0x22 || o[3] != 0x33 || o[4] != 0x44 || o[5] != 0x55 || o[6] || wp != o + 6) return 7;
  cp = b + 2; GETSHORT (s, cp); GETLONG (l, cp);
  if (s != 0x5678 || l != 0x9abcdef0UL || cp != b + 8) return 8;
  wp = o; PUTSHORT (0x0102, wp); PUTLONG (0x03040506, wp);
  if (o[0] != 1 || o[1] != 2 || o[2] != 3 || o[5] != 6 || wp != o + 6) return 9;
  return 0;
}
static_assert (niji_ns_values () == 0, "the arpa/nameser.h accessors' values");
"""
# Each byte-order macro is one statement: `if (c) M (...); else ...` compiles (a bare block would end the if).
NAMESER_STATEMENTS = """void niji_ns_stmt (int c, const unsigned char *p, unsigned char *w, unsigned int *s, unsigned long *l) {
  if (c) NS_GET16 (*s, p); else NS_GET32 (*l, p);
  if (c) NS_PUT16 (*s, w); else NS_PUT32 (*l, w);
  if (c) GETSHORT (*s, p); else GETLONG (*l, p);
  if (c) PUTSHORT (*s, w); else PUTLONG (*l, w);
}
"""


def nameser_accessor_units(ctx):
    """resolver(3) and RFC 1035's message layout (no spec clause beyond them; HEADERS-STAGE2-GAPS-E: keyutils'
    dns.afsdb.c counts answers with ns_msg_count): the ns_msg_*/ns_rr_* accessors and the byte-order macros of
    <arpa/nameser.h> exactly where the reference shows them; where shown, each accessor has the reference's type as a
    prvalue (C++11 on `decltype ((e))`: an lvalue would let `ns_msg_id (h) = 1` compile), the byte-order macros are
    single statements, and (C++17) the values: section counts by index, the empty owner name read as ".", big-endian
    reads and writes that keep the low bits and advance the pointer. Defects it catches: a missing accessor, an
    accessor that is an lvalue or has another type, a byte-order macro that is a brace block or reads little-endian."""
    out = ctx.exact("arpa/nameser.h", NAMESER_ACCESSORS)
    row = next((r for r in ctx.vis if r["header"] == "arpa/nameser.h" and r["name"] == "ns_msg_id"), None)
    if row is None:
        return out + [(ctx.ids[0], "nameser accessors", "#error visibility.tsv does not show ns_msg_id\n")]
    lang = {c["id"]: c["language"] for c in ctx.cfgs}
    for cid in expand(row["configs"], ctx.ids):
        cxx = lang[cid] == "c++"
        body = "#include <arpa/nameser.h>\nextern ns_msg niji_h;\nextern ns_rr niji_rr;\n"
        for i, (e, t) in enumerate(NAMESER_FORMS):
            # The empty name's "." is a `char[2]` in C, so the C conditional is `char *` (C probe of the reference).
            t = t.split(" (C: ")[0] if cxx else t.split(" (C: ")[-1].rstrip(")")
            same = (f"__is_same (__typeof__ ({e}), {t})" if cid == "cxx98" else f"__is_same (decltype (({e})), {t})") if cxx \
                else f"__builtin_types_compatible_p (__typeof__ ({e}), {t})"
            body += f"typedef char niji_ns_form{i}[{same} ? 1 : -1];\n"
        if cxx:
            body += "extern \"C\" {\n" + NAMESER_STATEMENTS + "}\n"
        else:
            body += NAMESER_STATEMENTS
        if cid == "cxx17":
            body += NAMESER_VALUES
        out.append((cid, "nameser accessors arpa/nameser.h", body))
    return out


def units(ctx):
    return nameser_enum_units(ctx) + nameser_accessor_units(ctx)
