#!/bin/sh
# CI-ENV-AUDIT prototype: print a sorted "key<TAB>value" manifest of the host inputs the gate reads.
# Read-only; no root. Each key is one input from results/ci-env-audit/README.md's inventory.
LC_ALL=C; export LC_ALL
kv() { printf '%s\t%s\n' "$1" "$2"; }
kv kernel.release "$(uname -r)"
kv kernel.version "$(uname -v)"
kv cpu.model "$(grep -m1 '^model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ *//')"
kv cpu.count "$(nproc)"
kv cpu.flags.sha "$(grep -m1 '^flags' /proc/cpuinfo | cut -d: -f2 | tr ' ' '\n' | sort -u | sha256sum | cut -c1-16)"
for f in avx2 avx512f avx512bw bmi2 erms fsrm shstk ibt user_shstk; do
  grep -m1 '^flags' /proc/cpuinfo | grep -qw "$f" && kv "cpu.flag.$f" 1 || kv "cpu.flag.$f" 0
done
kv os.release "$(. /etc/os-release; echo "$PRETTY_NAME")"
kv libc.reference "$(/usr/lib/x86_64-linux-gnu/libc.so.6 2>/dev/null | head -1)"
kv libc.reference.sha "$(sha256sum /usr/lib/x86_64-linux-gnu/libc.so.6 | cut -c1-16)"
kv ldso.reference.sha "$(sha256sum /lib64/ld-linux-x86-64.so.2 | cut -c1-16)"
for t in cc gcc g++ clang ld ld.bfd ld.gold ld.lld as ar objdump readelf nm python3 perl make strace gdb msgfmt gencat localedef iconv zic; do
  p=$(command -v $t 2>/dev/null)
  if [ -n "$p" ]; then v=$($t --version 2>&1 | head -1); kv "tool.$t" "$v"; else kv "tool.$t" ABSENT; fi
done
kv rust.toolchains "$(ls ~/.rustup/toolchains 2>/dev/null | tr '\n' ' ')"
kv cargo.nextest "$(cargo nextest --version 2>/dev/null | head -1 || echo ABSENT)"
kv locale.archive.sha "$(sha256sum /usr/lib/locale/locale-archive 2>/dev/null | cut -c1-16 || echo ABSENT)"
kv locale.archive.size "$(stat -c %s /usr/lib/locale/locale-archive 2>/dev/null || echo ABSENT)"
kv locale.dirs "$(ls -d /usr/lib/locale/*/ 2>/dev/null | wc -l)"
kv locale.list.sha "$(locale -a 2>/dev/null | sort | sha256sum | cut -c1-16)"
kv locale.count "$(locale -a 2>/dev/null | wc -l)"
kv i18n.locales.sha "$(ls /usr/share/i18n/locales 2>/dev/null | sha256sum | cut -c1-16)"
kv i18n.charmaps.sha "$(ls /usr/share/i18n/charmaps 2>/dev/null | sha256sum | cut -c1-16)"
kv zoneinfo.version "$(cat /usr/share/zoneinfo/tzdata.zi 2>/dev/null | head -1)"
kv gconv.modules.sha "$(sha256sum /usr/lib/x86_64-linux-gnu/gconv/gconv-modules.cache 2>/dev/null | cut -c1-16 || echo ABSENT)"
for f in /etc/resolv.conf /etc/hosts /etc/nsswitch.conf /etc/services /etc/protocols /etc/localtime /etc/ld.so.preload /etc/ld.so.cache; do
  if [ -e "$f" ]; then kv "etc.$(basename $f).sha" "$(cat "$f" 2>/dev/null | sha256sum | cut -c1-16)"; else kv "etc.$(basename $f).sha" ABSENT; fi
done
kv dpkg.count "$(dpkg-query -W -f '${Package}\n' 2>/dev/null | wc -l)"
kv dpkg.sha "$(dpkg-query -W -f '${Package}=${Version}\n' 2>/dev/null | sort | sha256sum | cut -c1-16)"
for p in libc6 libc6-dev libc-bin locales locales-all tzdata gcc g++ binutils libstdc++6 libgcc-s1 gettext; do
  kv "pkg.$p" "$(dpkg-query -W -f '${Version}' $p 2>/dev/null || echo ABSENT)"
done
for s in kernel.randomize_va_space vm.overcommit_memory vm.max_map_count vm.mmap_min_addr kernel.yama.ptrace_scope kernel.perf_event_paranoid kernel.unprivileged_userns_clone fs.mqueue.msg_max kernel.pid_max; do
  kv "sysctl.$s" "$(sysctl -n $s 2>/dev/null || echo ABSENT)"
done
kv thp.enabled "$(cat /sys/kernel/mm/transparent_hugepage/enabled 2>/dev/null)"
kv thp.defrag "$(cat /sys/kernel/mm/transparent_hugepage/defrag 2>/dev/null)"
kv fault_around_bytes "$(cat /sys/kernel/debug/fault_around_bytes 2>/dev/null || echo UNREADABLE)"
kv ulimit.stack "$(ulimit -s)"
kv ulimit.nofile "$(ulimit -n)"
kv ulimit.as "$(ulimit -v)"
kv tmp.fs "$(stat -f -c %T /tmp)"
kv home.fs "$(stat -f -c %T "$HOME")"
