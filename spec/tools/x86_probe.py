#!/usr/bin/env python3
"""Black-box probe of <sys/platform/x86.h> on the reference system; writes spec/abi/x86-features.tsv (SL-HEADERS-S16-2).

    python3 -I spec/tools/x86_probe.py [--out spec/abi/x86-features.tsv] [--table PROBE_TABLE.tsv]

What it does (plan/headers.md section 2.2: only the probe's own object and behaviour are read, never the reference's
header text):
* the candidate feature names, with leaf, sub-leaf, register and bit, come from the Intel SDM (Vol. 2A, instruction
  CPUID; the leaf's feature table is named per row; the five AMD-only bits: AMD APM Vol. 3, Appendix E) and from
  nothing else;
* each candidate is kept only if the reference's `CPU_FEATURE_ACTIVE(name)` and `CPU_FEATURE_PRESENT(name)` compile;
* a probe program defines its own `__x86_get_cpuid_feature_leaf` and records the index it is called with, then
  finds the one word and bit of `struct cpuid_feature` each macro tests (single-bit structures), and the type of
  the expression; a candidate whose index, register or bit differs from the SDM row is reported and dropped;
* the real function is called for each index and its four CPUID words compared with the CPUID instruction on the
  probe host for the SDM leaf of the index (`cpuid-match`); a leaf whose words are all zero there cannot be told
  apart from another zero leaf (`sdm-only`).
"""

# candidate feature names from the Intel SDM (Vol. 2A, CPUID): (leaf, subleaf, reg, [(bit, name), ...], table title)
C = []
def add(leaf, sub, reg, title, items):
    if not title.startswith("AMD"):
        title = "Intel SDM Vol. 2A, CPUID: " + title
    for bit, names in items:
        for n in names.split("|"):
            C.append((n, leaf, sub, reg, bit, title))
add(1,0,"ecx","CPUID.01H:ECX feature information",[(0,"SSE3"),(1,"PCLMULQDQ"),(2,"DTES64"),(3,"MONITOR"),(4,"DS_CPL"),(5,"VMX"),(6,"SMX"),(7,"EIST"),(8,"TM2"),(9,"SSSE3"),(10,"CNXT_ID"),(11,"SDBG"),(12,"FMA"),(13,"CMPXCHG16B"),(14,"XTPRUPDCTRL"),(15,"PDCM"),(17,"PCID"),(18,"DCA"),(19,"SSE4_1"),(20,"SSE4_2"),(21,"X2APIC"),(22,"MOVBE"),(23,"POPCNT"),(24,"TSC_DEADLINE"),(25,"AES"),(26,"XSAVE"),(27,"OSXSAVE"),(28,"AVX"),(29,"F16C"),(30,"RDRAND")])
add(1,0,"edx","CPUID.01H:EDX feature information",[(0,"FPU"),(1,"VME"),(2,"DE"),(3,"PSE"),(4,"TSC"),(5,"MSR"),(6,"PAE"),(7,"MCE"),(8,"CX8"),(9,"APIC"),(11,"SEP"),(12,"MTRR"),(13,"PGE"),(14,"MCA"),(15,"CMOV"),(16,"PAT"),(17,"PSE_36"),(18,"PSN"),(19,"CLFSH"),(21,"DS"),(22,"ACPI"),(23,"MMX"),(24,"FXSR"),(25,"SSE"),(26,"SSE2"),(27,"SS"),(28,"HTT"),(29,"TM"),(31,"PBE")])
add(7,0,"ebx","CPUID.(EAX=07H,ECX=0):EBX structured extended feature flags",[(0,"FSGSBASE"),(1,"TSC_ADJUST"),(2,"SGX"),(3,"BMI1"),(4,"HLE"),(5,"AVX2"),(6,"FDP_EXCPTN_ONLY"),(7,"SMEP"),(8,"BMI2"),(9,"ERMS"),(10,"INVPCID"),(11,"RTM"),(12,"RDT_M"),(13,"DEPR_FPU_CS_DS"),(14,"MPX"),(15,"RDT_A"),(16,"AVX512F"),(17,"AVX512DQ"),(18,"RDSEED"),(19,"ADX"),(20,"SMAP"),(21,"AVX512_IFMA"),(23,"CLFLUSHOPT"),(24,"CLWB"),(25,"TRACE"),(26,"AVX512PF"),(27,"AVX512ER"),(28,"AVX512CD"),(29,"SHA"),(30,"AVX512BW"),(31,"AVX512VL")])
add(7,0,"ecx","CPUID.(EAX=07H,ECX=0):ECX structured extended feature flags",[(0,"PREFETCHWT1"),(1,"AVX512_VBMI"),(2,"UMIP"),(3,"PKU"),(4,"OSPKE"),(5,"WAITPKG"),(6,"AVX512_VBMI2"),(7,"SHSTK"),(8,"GFNI"),(9,"VAES"),(10,"VPCLMULQDQ"),(11,"AVX512_VNNI"),(12,"AVX512_BITALG"),(13,"TME_EN"),(14,"AVX512_VPOPCNTDQ"),(16,"LA57"),(22,"RDPID"),(23,"KL"),(24,"BUS_LOCK_DETECT"),(25,"CLDEMOTE"),(27,"MOVDIRI"),(28,"MOVDIR64B"),(29,"ENQCMD"),(30,"SGX_LC"),(31,"PKS")])
add(7,0,"edx","CPUID.(EAX=07H,ECX=0):EDX structured extended feature flags",[(2,"AVX512_4VNNIW"),(3,"AVX512_4FMAPS"),(4,"FSRM"),(5,"UINTR"),(8,"AVX512_VP2INTERSECT"),(9,"SRBDS_CTRL"),(10,"MD_CLEAR"),(11,"RTM_ALWAYS_ABORT"),(13,"RTM_FORCE_ABORT"),(14,"SERIALIZE"),(15,"HYBRID"),(16,"TSXLDTRK"),(18,"PCONFIG"),(19,"ARCH_LBR"),(20,"IBT"),(22,"AMX_BF16"),(23,"AVX512_FP16"),(24,"AMX_TILE"),(25,"AMX_INT8"),(26,"IBRS_IBPB"),(27,"STIBP"),(28,"L1D_FLUSH"),(29,"ARCH_CAPABILITIES"),(30,"CORE_CAPABILITIES"),(31,"SSBD")])
add(7,1,"eax","CPUID.(EAX=07H,ECX=1):EAX structured extended feature flags",[(4,"AVX_VNNI"),(5,"AVX512_BF16"),(6,"LASS"),(7,"CMPCCXADD"),(10,"FZLRM"),(11,"FSRS"),(12,"FSRCS"),(19,"WRMSRNS"),(21,"AMX_FP16"),(22,"HRESET"),(23,"AVX_IFMA"),(26,"LAM"),(27,"MSRLIST")])
add(0xd,1,"eax","CPUID.(EAX=0DH,ECX=1):EAX processor extended state enumeration",[(0,"XSAVEOPT"),(1,"XSAVEC"),(2,"XGETBV_ECX_1"),(3,"XSAVES"),(4,"XFD")])
add(0x80000001,0,"ecx","CPUID.80000001H:ECX extended feature bits",[(0,"LAHF64_SAHF64|LAHF_SAHF"),(5,"LZCNT"),(8,"PREFETCHW")])
add(0x80000001,0,"ecx","AMD APM Vol. 3, Appendix E: CPUID Fn8000_0001_ECX",[(6,"SSE4A"),(11,"XOP"),(15,"LWP"),(16,"FMA4"),(21,"TBM")])
add(0x80000001,0,"edx","CPUID.80000001H:EDX extended feature bits",[(11,"SYSCALL_SYSRET|SYSCALL"),(20,"XD|NX"),(26,"PAGE1GB"),(27,"RDTSCP"),(29,"LM")])
add(0x80000007,0,"edx","CPUID.80000007H:EDX invariant TSC",[(8,"INVARIANT_TSC")])
add(0x80000008,0,"ebx","CPUID.80000008H:EBX",[(9,"WBNOINVD")])
add(0x14,0,"ebx","CPUID.(EAX=14H,ECX=0):EBX processor trace enumeration",[(4,"PTWRITE")])
add(0x19,0,"ebx","CPUID.19H:EBX key locker",[(0,"AESKLE"),(2,"WIDE_KL")])
add(6,0,"eax","CPUID.06H:EAX thermal and power management",[(1,"TURBO_BOOST"),(2,"ARAT"),(4,"PLN"),(6,"PTS"),(7,"HWP")])
add(0xd,0,"eax","CPUID.(EAX=0DH,ECX=0):EAX XCR0 supported bits",[(0,"XCR0_X87"),(1,"XCR0_SSE"),(2,"XCR0_AVX")])

import argparse, collections, os, shutil, subprocess, sys, tempfile

REG = ["eax", "ebx", "ecx", "edx"]


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def probe(d):
    names = sorted({c[0] for c in C})
    ok = []
    for n in names:
        open(f"{d}/n.c", "w").write(f"#include <sys/platform/x86.h>\nint f(void){{return CPU_FEATURE_ACTIVE({n}) + CPU_FEATURE_PRESENT({n});}}\n")
        if run(["gcc", "-std=gnu17", "-fsyntax-only", f"{d}/n.c"]).returncode == 0:
            ok.append(n)
    src = '#include <sys/platform/x86.h>\n#include <stdio.h>\n#include <string.h>\n#include <stddef.h>\n'
    src += 'static unsigned last; static struct cpuid_feature fake;\n'
    src += 'const struct cpuid_feature *__x86_get_cpuid_feature_leaf (unsigned int i) { last = i; return &fake; }\n'
    src += 'static void one(unsigned w, unsigned b) { memset(&fake, 0, sizeof fake); ((unsigned *)&fake)[w] = 1u << b; last = ~0u; }\n'
    src += 'int main(void) {\n  printf("LAYOUT\\t%zu\\t%zu\\t%zu\\t%zu\\n", sizeof(struct cpuid_feature), _Alignof(struct cpuid_feature), offsetof(struct cpuid_feature, cpuid_array), offsetof(struct cpuid_feature, active_array));\n'
    for n in ok:
        src += f'''  {{ int wa=-1,ba=-1,wp=-1,bp=-1; unsigned ia=~0u, ip=~0u; int na=0, np=0;
    for (unsigned w=0; w<8; w++) for (unsigned b=0; b<32; b++) {{
      one(w,b); int a = !!CPU_FEATURE_ACTIVE({n}); if (last != ~0u) ia = last; if (a) {{ wa=w; ba=b; na++; }}
      one(w,b); int p = !!CPU_FEATURE_PRESENT({n}); if (last != ~0u) ip = last; if (p) {{ wp=w; bp=b; np++; }}
    }}
    printf("FEAT\\t%s\\t%u\\t%u\\t%d\\t%d\\t%d\\t%d\\t%d\\t%d\\n", "{n}", ia, ip, wa, ba, na, wp, bp, np); }}
'''
    src += '  return 0;\n}\n'
    open(f"{d}/stub.c", "w").write(src)
    r = run(["gcc", "-std=gnu17", "-O0", f"{d}/stub.c", "-o", f"{d}/stub"])
    if r.returncode:
        sys.exit(r.stderr[:3000])
    out = run([f"{d}/stub"]).stdout
    leaves = sorted({(c[1], c[2]) for c in C})
    real = '#include <sys/platform/x86.h>\n#include <stdio.h>\n#include <cpuid.h>\nint main(void) {\n'
    real += '  for (unsigned i = 0; i < 16; i++) { const struct cpuid_feature *f = __x86_get_cpuid_feature_leaf(i); const unsigned *p = (const unsigned *)f; printf("IDX\\t%u", i); for (int k = 0; k < 8; k++) printf("\\t%08x", p[k]); printf("\\n"); }\n'
    for l, s in leaves:
        real += f'  {{ unsigned a=0,b=0,c=0,d=0; if (__get_cpuid_max({l}u & 0x80000000u, 0) >= {l}u) __cpuid_count({l}u, {s}u, a, b, c, d); printf("RAW\\t%x\\t%u\\t%08x\\t%08x\\t%08x\\t%08x\\n", {l}u, {s}u, a, b, c, d); }}\n'
    real += '  return 0;\n}\n'
    open(f"{d}/real.c", "w").write(real)
    r = run(["gcc", "-std=gnu17", "-O0", f"{d}/real.c", "-o", f"{d}/real"])
    if r.returncode:
        sys.exit(r.stderr[:3000])
    out += run([f"{d}/real"]).stdout
    # the type of the expressions (read back from the probe's own object)
    ty = '#include <sys/platform/x86.h>\nint a = _Generic(CPU_FEATURE_ACTIVE(SSE2), int:1, unsigned int:2, _Bool:3, default:9);\nint p = _Generic(CPU_FEATURE_PRESENT(SSE2), int:1, unsigned int:2, _Bool:3, default:9);\n'
    open(f"{d}/ty.c", "w").write(ty + 'int main(void){return a*10+p;}\n')
    run(["gcc", "-std=gnu17", f"{d}/ty.c", "-o", f"{d}/ty"])
    out += "TYPE\t%d\n" % run([f"{d}/ty"]).returncode
    return out, names, ok


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default="spec/abi/x86-features.tsv")
    ap.add_argument("--table", help="also write the probe table (every index, word, bit, layout) here")
    args = ap.parse_args()
    d = tempfile.mkdtemp(prefix="x86probe.", dir=os.path.expanduser("~"))
    try:
        out, names, ok = probe(d)
    finally:
        shutil.rmtree(d)
    cm = {c[0]: c for c in C}
    idx, raw, feat, layout, ty = {}, {}, {}, None, None
    for l in out.splitlines():
        f = l.split("\t")
        if f[0] == "IDX": idx[int(f[1])] = f[2:]
        elif f[0] == "RAW": raw[(int(f[1], 16), int(f[2]))] = f[3:]
        elif f[0] == "FEAT": feat[f[1]] = f[2:]
        elif f[0] == "LAYOUT": layout = f[1:]
        elif f[0] == "TYPE": ty = f[1]
    rows, dropped = [], []
    byidx = collections.defaultdict(set)
    for n in sorted(feat, key=lambda n: (int(feat[n][0]) & 0xffff, cm[n][4], n)):
        ia, ip, wa, ba, na, wp, bp, np_ = (int(x) for x in feat[n])
        _, leaf, sub, reg, bit, title = cm[n]
        if not (np_ == 1 and wp < 4 and REG[wp] == reg and bp == bit and ip < 0xffff):
            dropped.append((n, feat[n]))
            continue
        if na == 1 and ia == ip and wa == wp + 4 and ba == bp:
            active = "yes"
        elif na == 0:
            active = "never"
        else:
            dropped.append((n, feat[n]))
            continue
        byidx[ip].add((leaf, sub))
        rows.append((n, ip, leaf, sub, reg, bit, active, title))
    for i, ls in byidx.items():
        assert len(ls) == 1, (i, ls)
    check = {}
    for i, ls in byidx.items():
        leaf, sub = next(iter(ls))
        r = raw[(leaf, sub)]
        words = idx[i][:4]
        check[i] = "cpuid-match" if words == r and any(int(w, 16) for w in r) else "sdm-only"
    with open(args.out, "w", newline="\n") as f:
        f.write("# CPU_FEATURE_ACTIVE / CPU_FEATURE_PRESENT names of <sys/platform/x86.h> (spec/tools/x86_probe.py).\n")
        f.write(f"# struct cpuid_feature: size {layout[0]}, align {layout[1]}, cpuid_array at {layout[2]}, active_array at {layout[3]} (layouts.tsv);\n")
        f.write("# both macros have type _Bool (the type probe exits 33 for _Bool and _Bool; this run: %s). index: argument of __x86_get_cpuid_feature_leaf;\n" % ty)
        f.write("# reg/bit: the register word and bit of the CPUID result, the same in cpuid_array and active_array; active=never: the\n")
        f.write("# macro CPU_FEATURE_ACTIVE is false for every structure; leaf_check: cpuid-match = the words of the real function for that\n")
        f.write("# index equal the CPUID instruction at leaf/subleaf on the probe host, sdm-only = those words are zero there.\n")
        f.write("name\tindex\tleaf\tsubleaf\treg\tbit\tactive\tleaf_check\tsource\n")
        for n, i, leaf, sub, reg, bit, active, title in rows:
            f.write(f"{n}\t{i}\t0x{leaf:x}\t{sub}\t{reg}\t{bit}\t{active}\t{check[i]}\t{title}\n")
    if args.table:
        with open(args.table, "w", newline="\n") as f:
            f.write("name\tindex_active\tindex_present\tword_active\tbit_active\tcount_active\tword_present\tbit_present\tcount_present\n")
            for n in sorted(feat):
                f.write(n + "\t" + "\t".join(feat[n]) + "\n")
            f.write("# layout (size, align, cpuid_array, active_array): " + " ".join(layout) + "\n")
    print(f"accepted {len(ok)} of {len(names)} candidates; rejected: {' '.join(n for n in names if n not in ok)}")
    print(f"written {len(rows)} rows to {args.out}; dropped {len(dropped)}: {dropped}")
    print("indices:", {i: (hex(next(iter(l))[0]), next(iter(l))[1], check[i]) for i, l in sorted(byidx.items())})


main()
