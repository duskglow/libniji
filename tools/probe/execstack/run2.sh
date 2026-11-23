#!/bin/sh
# run2.sh DIR: build and run the second executable-stack probe in DIR (sources es2.c there):
# which stacks an executable-stack request reaches (COMPAT19-TUNABLE).
set -u
cd "$1" || exit 2
printf 'int es_lib(void) { return 1; }\n' > lib.c
cc -O1 -shared -fPIC -Wl,-z,execstack -o libx.so lib.c || exit 3
cc -O1 -o es2 es2.c -ldl -lpthread || exit 3
cc -O1 -o es2_dep es2.c -ldl -lpthread -Wl,--no-as-needed ./libx.so -Wl,-rpath,"$PWD" || exit 3
echo "== es2, dlopen libx.so"; timeout 20 ./es2 ./libx.so; echo "rc $?"
echo "== es2_dep (startup dependency asks), no dlopen"; timeout 20 ./es2_dep; echo "rc $?"
ldd --version | head -1
