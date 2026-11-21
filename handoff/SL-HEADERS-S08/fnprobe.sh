#!/bin/bash
# fnmembers.tsv check (ruling #6448): in C++, assigning a function of exactly the row's type to the member compiles, and the
# member's type is exactly that function-pointer type; a function with one wrong parameter type does not compile.
# usage: fnprobe.sh ref | niji ; run from ~/work/SL-HEADERS-S08-3
cd ~/work/SL-HEADERS-S08-3 || exit 2
case "$1" in
  ref) FL="" ;;
  niji) FL="-nostdinc -isystem $HOME/w/s2/libc-include -isystem $(gcc -print-file-name=include)" ;;
esac
HDR='#include <signal.h>
#include <pthread.h>
'
fail=0
t() { # name, good-source, bad-source
  printf '%s%s\n' "$HDR" "$2" > fp_good.cc
  printf '%s%s\n' "$HDR" "$3" > fp_bad.cc
  g=ok; g++ -std=c++17 $FL -fsyntax-only -Wall -Werror fp_good.cc 2>/dev/null || g=FAIL
  b=rejected; g++ -std=c++17 $FL -fsyntax-only fp_bad.cc 2>/dev/null && b=ACCEPTED
  echo "$1 exact-type=$g wrong-type=$b"
  [ "$g" = ok ] && [ "$b" = rejected ] || fail=1
}
t "struct sigaction.sa_sigaction" \
 'void f(int, siginfo_t *, void *); struct sigaction a; void u() { a.sa_sigaction = f; static_assert(__is_same(decltype(a.sa_sigaction), void (*)(int, siginfo_t *, void *)), ""); }' \
 'void f(int, void *, void *); struct sigaction a; void u() { a.sa_sigaction = f; }'
t "struct sigaction.sa_restorer" \
 'void f(void); struct sigaction a; void u() { a.sa_restorer = f; static_assert(__is_same(decltype(a.sa_restorer), void (*)(void)), ""); }' \
 'void f(int); struct sigaction a; void u() { a.sa_restorer = f; }'
t "struct sigevent._function" \
 'void f(union sigval); struct sigevent e; void u() { e.sigev_notify_function = f; static_assert(__is_same(decltype(e.sigev_notify_function), void (*)(union sigval)), ""); }' \
 'void f(int); struct sigevent e; void u() { e.sigev_notify_function = f; }'
t "sigevent_t._function" \
 'void f(union sigval); sigevent_t e; void u() { e.sigev_notify_function = f; static_assert(__is_same(decltype(e.sigev_notify_function), void (*)(union sigval)), ""); }' \
 'void f(int); sigevent_t e; void u() { e.sigev_notify_function = f; }'
t "struct _pthread_cleanup_buffer.__routine" \
 'void f(void *); struct _pthread_cleanup_buffer b; void u() { b.__routine = f; static_assert(__is_same(decltype(b.__routine), void (*)(void *)), ""); }' \
 'void f(int *); struct _pthread_cleanup_buffer b; void u() { b.__routine = f; }'
echo "fnprobe $1 fail=$fail"
exit $fail
