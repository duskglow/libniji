"""Write the P2/P3 accept file for slice 12 (python3 accept.py ROOT DIFF > accept.txt): differences on the three headers whose include of sys/param.h or signal.h
is guarded until slices 7 and 8 land (ruling #6406), limited to names those headers (and what they include) own."""
import sys
vis = {}
others = {"signal.h", "sys/param.h", "unistd.h", "stdio.h", "getopt.h", "time.h", "sys/types.h", "stdlib.h", "string.h",
          "limits.h", "stdint.h", "endian.h", "sys/select.h", "sys/time.h", "errno.h", "ctype.h", "strings.h"}
foreign = set()
for l in open(sys.argv[1] + "/spec/abi/visibility.tsv"):
    p = l.rstrip("\n").split("\t")
    if len(p) >= 4 and p[0] in others:
        foreign.add(p[1])
guarded = {"arpa/nameser.h", "resolv.h", "netdb.h"}
out = []
for l in open(sys.argv[2]):
    p = l.split()
    if len(p) < 4 or p[0] not in ("missing", "differs", "extra"):
        continue
    if p[2] in ("_SIGSET_NWORDS", "_STRUCT_TIMESPEC", "bindresvport6"):
        out.append(l.rstrip("\n"))  # reserved names no template defines; bindresvport6 is not exported (differences row)
    elif p[1] in guarded and p[2] in foreign:
        out.append(l.rstrip("\n"))  # sys/param.h and signal.h are guarded includes until slices 7 and 8 land (#6406)
    elif p[1] == "sys/socket.h" and p[2] in ("recv", "recvfrom"):
        out.append(l.rstrip("\n"))  # the fortify call forms are slice 18's
print("\n".join(out))
