#!/usr/bin/env python3
"""PERF-GAP-STRUCTURE-AUDIT probe 5. (1) cost per imported function: programs with 0/50/200 never-called PLT imports,
lazy and -z now, both kits. (2) real distribution binaries (built for the reference) run by each loader, invoked
the same way (ld.so --library-path ... program), outputs compared. (3) page-table and resident sizes at the end of
main (/proc/self/status). Counters only; nothing of the reference is symbolised."""
import os
import re
import statistics
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
SHA = "0495c4684330b90a6cecc02271ba1429296536ce"
P = os.path.expanduser("~/wl/products/" + SHA)
ROLE = os.path.expanduser("~/w/s2")
OUT = os.path.join(HERE, "out5")
NLD = P + "-release-dyn/ld-linux-x86-64.so.2"
NLIB = P + "-release-dyn"
RLD = "/lib64/ld-linux-x86-64.so.2"
SYSLIB = "/lib/x86_64-linux-gnu"
ENV = {"PATH": "/usr/bin:/bin", "HOME": "/tmp", "LANG": "C.UTF-8"}
EV = "instructions:u,instructions:k,page-faults,cycles:u,cycles:k"


def sh(cmd, env=None, timeout=900, **kw):
    return subprocess.run(cmd, env=env, capture_output=True, text=True, timeout=timeout, **kw)


def cc(kit, src, out, extra=()):
    if kit == "ref":
        r = sh(["gcc", "-O2", src, "-o", out, *extra])
    else:
        r = sh(["sh", ROLE + "/validation/wl_dyncc.sh", "-O2", src, "-o", out, *extra],
               env=dict(os.environ, WL_DYN_PRODUCTS=NLIB))
    if r.returncode:
        print("BUILD-FAIL", out, r.stderr[-700:])
        return False
    return True


def med(argv, n=7, env=ENV):
    runs, rc, out = [], 0, ""
    for _ in range(n):
        r = sh(["perf", "stat", "-x,", "-e", EV, "--"] + argv, env=env)
        d = {}
        for ln in r.stderr.splitlines():
            f = ln.split(",")
            if len(f) > 3:
                try:
                    d[f[2]] = float(f[0])
                except ValueError:
                    pass
        runs.append(d)
        rc, out = r.returncode, r.stdout
    m = {k: statistics.median(x.get(k, float("nan")) for x in runs) for k in EV.split(",")}
    return m, rc, out


def fmt(m):
    return (f"iu {m['instructions:u']:9.0f}  ik {m['instructions:k']:9.0f}  pf {m['page-faults']:4.0f}"
            f"  cu {m['cycles:u']:9.0f}  ck {m['cycles:k']:9.0f}")


def dynfuncs(path):
    out = sh(["readelf", "--dyn-syms", "-W", path]).stdout
    s = set()
    for ln in out.splitlines():
        f = ln.split()
        if len(f) >= 8 and f[3] == "FUNC" and f[4] == "GLOBAL" and f[5] == "DEFAULT" and f[6] != "UND" and "@@" in f[7]:
            s.add(f[7].split("@")[0])
    return s


def main():
    os.makedirs(OUT, exist_ok=True)
    os.chdir(OUT)
    print("(1) imported functions never called (PLT entries only), env = PATH HOME LANG")
    ref_libc = os.path.realpath(SYSLIB + "/libc.so.6")
    names = sorted(n for n in (dynfuncs(ref_libc) & dynfuncs(NLIB + "/libc.so.6"))
                   if re.match(r"^[a-z][a-z0-9_]*$", n) and n not in ("main", "exit", "abort"))
    print(f"    {len(names)} candidate names; the first 200 used")
    for k in (0, 50, 200):
        with open(f"imp{k}.c", "w") as f:
            for i, nm in enumerate(names[:k]):
                f.write(f'extern void f{i}(void) __asm__("{nm}");\n')
            f.write("int main(int argc, char **argv) {\n  (void)argv;\n  if (argc > 1000) {\n")
            for i in range(k):
                f.write(f"    f{i}();\n")
            f.write("  }\n  return 0;\n}\n")
    res = {}
    for kit in ("ref", "niji"):
        for mode, extra in (("lazy", ("-Wl,-z,lazy",)), ("now", ("-Wl,-z,now",))):
            for k in (0, 50, 200):
                b = f"imp{k}.{kit}-{mode}"
                if not cc(kit, f"imp{k}.c", b, extra):
                    continue
                m, rc, _ = med(["./" + b])
                res[(kit, mode, k)] = m
                flags = re.findall(r"\((?:FLAGS|FLAGS_1)\)\s+(.*)", sh(["readelf", "-dW", b]).stdout)
                print(f"    {b:18} rc {rc} {fmt(m)}  dyn flags {flags}")
    for kit in ("ref", "niji"):
        for mode in ("lazy", "now"):
            a, z = res.get((kit, mode, 200)), res.get((kit, mode, 0))
            if a and z:
                print(f"    per import, {kit} {mode}: user instructions {(a['instructions:u'] - z['instructions:u']) / 200:7.1f}"
                      f"  user cycles {(a['cycles:u'] - z['cycles:u']) / 200:7.1f}"
                      f"  faults {(a['page-faults'] - z['page-faults']) / 200:5.3f}")
    if os.environ.get("PGSA_ONLY_IMPORTS"):
        print("\n== done (imports only)")
        return

    print("\n(2) distribution binaries (built for the reference), each loader invoked explicitly, env = PATH HOME LANG")
    with open("words.txt", "w") as f:
        f.write("\n".join(["pear", "apple", "Zebra", "banana", "cherry", "apple"] * 20) + "\n")
    progs = [("true", ["/bin/true"]), ("cat", ["/bin/cat", "/etc/hostname"]), ("ls", ["/bin/ls", "/"]),
             ("sort", ["/usr/bin/sort", "words.txt"]), ("date", ["/bin/date", "-d", "@0", "+%F"]),
             ("wc", ["/usr/bin/wc", "-l", "words.txt"]), ("sed", ["/bin/sed", "s/a/b/", "words.txt"]),
             ("grep", ["/bin/grep", "-c", "apple", "words.txt"]), ("printf", ["/usr/bin/printf", "%s %d\\n", "x", "42"])]
    print("    needed libraries: " + ", ".join(
        f"{n}: {len(re.findall('NEEDED', sh(['readelf', '-dW', a[0]]).stdout))}" for n, a in progs))
    for name, argv in progs:
        mr, rcr, outr = med([RLD, "--library-path", SYSLIB] + argv)
        mn, rcn, outn = med([NLD, "--library-path", NLIB + ":" + SYSLIB] + argv)
        same = "same output" if (outr == outn and rcr == rcn) else f"OUTPUT DIFFERS (rc {rcr}/{rcn})"
        print(f"    {name:7} ref  {fmt(mr)}")
        print(f"    {name:7} niji {fmt(mn)}  {same}")
        print(f"    {name:7} ratio iu {mn['instructions:u'] / mr['instructions:u']:.2f}  ik {mn['instructions:k'] / mr['instructions:k']:.2f}"
              f"  pf {mn['page-faults'] / mr['page-faults']:.2f}  cu+ck {(mn['cycles:u'] + mn['cycles:k']) / (mr['cycles:u'] + mr['cycles:k']):.2f}")

    print("\n(3) /proc/self/status at the end of main (empty program)")
    with open("status.c", "w") as f:
        f.write('#include <fcntl.h>\n#include <unistd.h>\nint main(void){static char b[8192];int fd=open("/proc/self/status",O_RDONLY);'
                'long n=read(fd,b,sizeof b);if(n>0)write(1,b,(unsigned long)n);return 0;}\n')
    for kit in ("ref", "niji"):
        if cc(kit, "status.c", "status." + kit):
            out = sh(["./status." + kit], env=ENV).stdout
            keep = [ln.replace("\t", " ") for ln in out.splitlines() if ln.split(":")[0] in
                    ("VmSize", "VmRSS", "RssAnon", "RssFile", "VmPTE", "VmData", "VmStk")]
            print(f"    {kit}: " + " | ".join(" ".join(k.split()) for k in keep))
    print("\n== done")


main()
