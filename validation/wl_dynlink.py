#!/usr/bin/env python3
# LAB-ONLY: this script drives the project's own test hosts and is not needed to build or test libniji.
# ELF helpers of the libniji-dynamic workload kit (WL-DYNAMIC-KIT; validation/wl_lock.sh wl_dyn_layout, wl_dyncc.sh).
#
#   wl_dynlink.py bypath SRC DST        copy of shared object SRC whose DT_SONAME entry is renamed to an unused tag, so
#                                       a link against DST records DST's path as DT_NEEDED (as conformance/run.sh's
#                                       bypath and crates/core/tests/crt_products.rs by_path do)
#   wl_dynlink.py retarget BIN DIR      in program BIN, rewrite (in place, same file offsets) every DT_NEEDED string
#                                       DIR/lib/<lib>-by-path.so to DIR/<lib>.so.6, the shipped file with its DT_SONAME,
#                                       and check PT_INTERP is DIR/ld-linux-x86-64.so.2
#
# Why retarget: libniji's loader opens libc.so.6 and libm.so.6 only from LD_LIBRARY_PATH or its install directory,
# never RUNPATH; an absolute DT_NEEDED is loaded by path. The runtime object must keep its DT_SONAME: libm.so.6's own
# bare DT_NEEDED libc.so.6 is matched against the loaded objects' DT_SONAME. The new string is shorter than the old;
# the rest is zero-filled, so no offset in the file moves. Refused (exit 1) when any other .dynstr reference
# (a symbol, a version name, another dynamic entry) points into the middle of the rewritten string (string tail
# sharing), or when a by-path DT_NEEDED is left.
import struct
import sys

UNUSED_TAG = 0x6FFFFDF8
DT_NEEDED, DT_SONAME, DT_RPATH, DT_RUNPATH = 1, 14, 15, 29
SHT_DYNAMIC, SHT_DYNSYM, SHT_VERNEED, SHT_VERDEF = 6, 11, 0x6FFFFFFE, 0x6FFFFFFD
PT_INTERP = 3


def die(msg):
    sys.stderr.write("wl_dynlink: " + msg + "\n")
    sys.exit(1)


def sections(b):
    if b[:4] != b"\x7fELF" or b[4] != 2 or b[5] != 1:
        die("not a little-endian ELF64 file")
    shoff, = struct.unpack_from("<Q", b, 0x28)
    shentsize, shnum = struct.unpack_from("<HH", b, 0x3A)
    out = []
    for i in range(shnum):
        name, typ, _flags, _addr, off, size, link, info, _al, ent = struct.unpack_from("<IIQQQQIIQQ", b, shoff + i * shentsize)
        out.append({"type": typ, "off": off, "size": size, "link": link, "info": info, "ent": ent})
    return out


def dynamic(b, secs):
    d = [s for s in secs if s["type"] == SHT_DYNAMIC]
    if len(d) != 1:
        die("want one SHT_DYNAMIC section, found %d" % len(d))
    d = d[0]
    ents = []
    for k in range(d["size"] // 16):
        tag, val = struct.unpack_from("<qQ", b, d["off"] + 16 * k)
        ents.append((d["off"] + 16 * k, tag, val))
        if tag == 0:
            break
    return d, ents


def bypath(src, dst):
    b = bytearray(open(src, "rb").read())
    _, ents = dynamic(b, sections(b))
    hits = [off for off, tag, _ in ents if tag == DT_SONAME]
    if len(hits) != 1:
        die("%s: want one DT_SONAME, found %d" % (src, len(hits)))
    struct.pack_into("<q", b, hits[0], UNUSED_TAG)
    with open(dst + ".tmp", "wb") as f:
        f.write(b)
    import os
    os.chmod(dst + ".tmp", 0o755)
    os.replace(dst + ".tmp", dst)


def str_refs(b, secs, dsec, ents):
    """Every offset into the dynamic string table that the file uses, with what uses it."""
    refs = []
    for _, tag, val in ents:
        if tag in (DT_NEEDED, DT_SONAME, DT_RPATH, DT_RUNPATH):
            refs.append((val, "dynamic tag %d" % tag))
    for s in secs:
        if s["type"] == SHT_DYNSYM and s["link"] == dsec["link"]:
            for k in range(s["size"] // 24):
                refs.append((struct.unpack_from("<I", b, s["off"] + 24 * k)[0], "symbol %d" % k))
        if s["type"] == SHT_VERNEED and s["link"] == dsec["link"]:
            p, n = s["off"], 0
            while n < max(s["info"], 1):
                _ver, cnt, vfile, aux, nxt = struct.unpack_from("<HHIII", b, p)
                refs.append((vfile, "verneed file"))
                a = p + aux
                for _ in range(cnt):
                    _h, _f, _o, name, anext = struct.unpack_from("<IHHII", b, a)
                    refs.append((name, "verneed name"))
                    if anext == 0:
                        break
                    a += anext
                n += 1
                if nxt == 0:
                    break
                p += nxt
        if s["type"] == SHT_VERDEF and s["link"] == dsec["link"]:
            die("a program with version definitions: not expected here")
    return refs


def cstr(b, off):
    e = b.index(b"\0", off)
    return bytes(b[off:e])


def retarget(path, d):
    d = d.rstrip("/")
    b = bytearray(open(path, "rb").read())
    secs = sections(b)
    dsec, ents = dynamic(b, secs)
    strtab = secs[dsec["link"]]
    refs = str_refs(b, secs, dsec, ents)
    want = {}
    for lib in ("libc", "libm"):
        want[("%s/lib/%s-by-path.so" % (d, lib)).encode()] = ("%s/%s.so.6" % (d, lib)).encode()
    done = []
    for _, tag, val in ents:
        if tag != DT_NEEDED:
            continue
        old = cstr(b, strtab["off"] + val)
        if old not in want:
            if b"-by-path" in old:
                die("%s: unexpected by-path DT_NEEDED %s" % (path, old.decode()))
            continue
        new = want[old]
        for r, what in refs:
            if val < r < val + len(old):
                die("%s: %s points inside DT_NEEDED %s; not rewritten" % (path, what, old.decode()))
        p = strtab["off"] + val
        b[p:p + len(old)] = new + b"\0" * (len(old) - len(new))
        done.append(new.decode())
    for _, tag, val in ents:
        if tag == DT_NEEDED and b"-by-path" in cstr(b, strtab["off"] + val):
            die("%s: a by-path DT_NEEDED is left" % path)
    if not any(x.endswith("/libc.so.6") for x in done):
        die("%s: no by-path libc DT_NEEDED to retarget (was it linked with wl_dyncc.sh?)" % path)
    phoff, = struct.unpack_from("<Q", b, 0x20)
    phentsize, phnum = struct.unpack_from("<HH", b, 0x36)
    interp = None
    for i in range(phnum):
        typ, _fl, off, _va, _pa, filesz = struct.unpack_from("<IIQQQQ", b, phoff + i * phentsize)
        if typ == PT_INTERP:
            interp = bytes(b[off:off + filesz]).rstrip(b"\0").decode()
    if interp != d + "/ld-linux-x86-64.so.2":
        die("%s: PT_INTERP is %s, want %s/ld-linux-x86-64.so.2" % (path, interp, d))
    with open(path + ".rt", "wb") as f:
        f.write(b)
    import os
    os.chmod(path + ".rt", os.stat(path).st_mode & 0o7777)
    os.replace(path + ".rt", path)
    print("wl_dynlink: %s: DT_NEEDED %s, interpreter %s" % (path, " ".join(done), interp))


def main(argv):
    if len(argv) == 4 and argv[1] == "bypath":
        bypath(argv[2], argv[3])
    elif len(argv) == 4 and argv[1] == "retarget":
        retarget(argv[2], argv[3])
    else:
        sys.stderr.write("usage: wl_dynlink.py bypath SRC DST | retarget BIN DIR\n")
        sys.exit(2)


if __name__ == "__main__":
    main(sys.argv)
