#!/usr/bin/env python3
"""P10 for slice 7 (plan/headers.md section 3): the macros of <sys/select.h>, <sys/types.h> and <sys/sysmacros.h>,
differentially against the system's own headers.

    python3 spec/tools/header_p10.py [--include DIR] [--work DIR]

One C program (printed below as PROGRAM) is compiled twice, at -O0 and -O2: against the system's headers and against
libniji's (`-nostdinc -isystem DIR -isystem <the compiler's include>`, DIR = libc-include/ by default), both linked
with the system's C library (the headers' ABI is what is under test). Both must print the same lines:

* FD_SET, FD_ISSET, FD_CLR, FD_ZERO over every descriptor 0..1023 (the words that change, and the answers);
* major, minor and makedev over the edge values and 20000 pseudo-random 64-bit device numbers, and makedev (major, minor)
  against the original.

And the program built on libniji's headers must bind to no `gnu_dev_*` symbol (a call is expanded in place) and must
bind to `__fdelt_chk` (the descriptor range check). Prints one FAIL line per difference and `header_p10: N failed`;
exit 1 when any failed. Needs gcc only.
"""
import argparse
import os
import subprocess
import sys
import tempfile

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))

PROGRAM = r"""
#include <stdio.h>
#include <sys/select.h>
#include <sys/sysmacros.h>

int main (void)
{
  fd_set s;
  for (int fd = 0; fd < 1024; fd++)
    {
      FD_ZERO (&s);
      FD_SET (fd, &s);
      int set = FD_ISSET (fd, &s), next = FD_ISSET ((fd + 1) % 1024, &s);
      printf ("set %d:", fd);
      for (unsigned i = 0; i < sizeof s / sizeof (long); i++)
        {
          long w = *((long *) &s + i);
          if (w)
            printf (" [%u]=%lx", i, w);
        }
      FD_CLR (fd, &s);
      int clear = FD_ISSET (fd, &s);
      printf (" isset=%d next=%d after-clr=%d\n", set, next, clear);
    }
  unsigned long long x = 88172645463325252ULL;
  for (int i = 0; i < 20003; i++)
    {
      unsigned long long d;
      if (i < 3)
        d = i == 0 ? 0 : i == 1 ? ~0ULL : 0x0123456789abcdefULL;
      else
        {
          x ^= x << 13; x ^= x >> 7; x ^= x << 17;
          d = x;
        }
      unsigned int ma = major (d), mi = minor (d);
      printf ("dev %llx: %x %x %llx\n", d, ma, mi, (unsigned long long) makedev (ma, mi));
    }
  return 0;
}
"""


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--include", default=os.path.join(ROOT, "libc-include"))
    ap.add_argument("--work", default="")
    ap.add_argument("--uapi", default="", help="a kernel UAPI tree (target/niji/uapi): also run P8 for sys/xattr.h")
    args = ap.parse_args()
    work = args.work or tempfile.mkdtemp(prefix="p10-")
    os.makedirs(work, exist_ok=True)
    src = os.path.join(work, "p10.c")
    with open(src, "w") as f:
        f.write(PROGRAM)
    gccinc = subprocess.run(["gcc", "-print-file-name=include"], capture_output=True, text=True, check=True).stdout.strip()
    failed = 0
    for opt in ("-O0", "-O2"):
        outs = {}
        for side, flags in (("system", []), ("libniji", ["-nostdinc", "-isystem", args.include, "-isystem", gccinc])):
            exe = os.path.join(work, f"p10-{side}{opt}")
            r = subprocess.run(["gcc", opt, "-Wall", "-Wextra", "-Werror", *flags, src, "-o", exe], capture_output=True, text=True)
            if r.returncode:
                print(f"FAIL P10 {side} {opt}: does not compile: {r.stderr.strip().splitlines()[:3]}")
                failed += 1
                continue
            run = subprocess.run([exe], capture_output=True, text=True)
            if run.returncode:
                print(f"FAIL P10 {side} {opt}: exit {run.returncode}")
                failed += 1
            outs[side] = run.stdout
            if side == "libniji":
                und = subprocess.run(["nm", "-u", exe], capture_output=True, text=True).stdout
                if "gnu_dev_" in und:
                    print(f"FAIL P10 libniji {opt}: binds to a gnu_dev_* symbol")
                    failed += 1
                if "__fdelt_chk" not in und:
                    print(f"FAIL P10 libniji {opt}: does not bind to __fdelt_chk")
                    failed += 1
        for macro in ("FD_SET (fd, &s)", "FD_CLR (fd, &s)", "(void) FD_ISSET (fd, &s)"):
            # Each descriptor macro on its own: one that skips the range check still gives the same answers for every
            # valid descriptor, so only its own undefined symbol shows it.
            msrc = os.path.join(work, "p10-macro.c")
            with open(msrc, "w") as f:
                f.write("#include <sys/select.h>\nint main (int argc, char **argv) { fd_set s = {{0}}; int fd = argc; %s; return (int) (long) argv; }\n" % macro)
            mexe = os.path.join(work, f"p10-macro{opt}")
            r = subprocess.run(["gcc", opt, "-nostdinc", "-isystem", args.include, "-isystem", gccinc, msrc, "-o", mexe], capture_output=True, text=True)
            if r.returncode:
                print(f"FAIL P10 libniji {opt}: {macro} does not compile: {r.stderr.strip().splitlines()[:3]}")
                failed += 1
            elif "__fdelt_chk" not in subprocess.run(["nm", "-u", mexe], capture_output=True, text=True).stdout:
                print(f"FAIL P10 libniji {opt}: {macro} does not bind to __fdelt_chk")
                failed += 1
        if len(outs) == 2 and outs["system"] != outs["libniji"]:
            a, b = outs["system"].splitlines(), outs["libniji"].splitlines()
            diff = [(i, x, y) for i, (x, y) in enumerate(zip(a, b)) if x != y]
            print(f"FAIL P10 {opt}: {len(diff)} of {len(a)} lines differ (lines {len(a)} vs {len(b)}); first: {diff[0] if diff else '-'}")
            failed += 1
    if args.uapi:
        failed += p8(args.include, args.uapi, gccinc, work)
    print(f"header_p10: {failed} failed")
    return 1 if failed else 0


P8_SOURCE = r"""
#include <%s>
#include <%s>
_Static_assert (XATTR_CREATE == 1 && XATTR_REPLACE == 2, "xattr flags");
int main (void) { return 0; }
"""


def p8(include, uapi, gccinc, work):
    """P8 (kernel UAPI, ruling R-H1/R-H2): sys/xattr.h and the kernel's linux/xattr.h in both include orders, with and
    without _GNU_SOURCE, compile together with -Wall -Werror and agree on XATTR_CREATE and XATTR_REPLACE."""
    failed = 0
    for order in (("sys/xattr.h", "linux/xattr.h"), ("linux/xattr.h", "sys/xattr.h")):
        for define in ([], ["-D_GNU_SOURCE"]):
            src = os.path.join(work, "p8.c")
            with open(src, "w") as f:
                f.write(P8_SOURCE % order)
            r = subprocess.run(["gcc", "-std=gnu17", "-Wall", "-Wextra", "-Werror", "-nostdinc", "-isystem", include, "-isystem", uapi,
                                "-isystem", gccinc, *define, "-fsyntax-only", src], capture_output=True, text=True)
            if r.returncode:
                print(f"FAIL P8 {' then '.join(order)} {' '.join(define) or 'default'}: {r.stderr.strip().splitlines()[:3]}")
                failed += 1
    return failed


if __name__ == "__main__":
    sys.exit(main())
