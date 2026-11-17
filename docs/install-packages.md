# Installing the reference packages

libniji has reference Debian packages for Debian 13 (trixie) and Devuan 6 (excalibur), amd64.
They install libniji **beside** the system's C library: glibc (`libc6`, `libc6-dev`) and every
existing program are left as they are, and purging the packages leaves the system as it was.
Replacing `libc6` (libniji as the system's C library) is the separate system flavour at the end of
this page, for Yohane Linux.

## What they are

| Package | Like Debian's | Contents |
|---|---|---|
| `niji6` | `libc6` | the loader `ld-linux-x86-64.so.2` and the shared libraries (`libc.so.6`, `libm.so.6`, `libpthread.so.0`, `libdl.so.2`, `librt.so.1`, `libutil.so.1`, `libanl.so.1`, `libnsl.so.1`, `libc_malloc_debug.so.0`, `libresolv.so.2`, `libBrokenLocale.so.1`) |
| `niji6-dev` | `libc6-dev` | start files (`crt1.o`, `Scrt1.o`, `rcrt1.o`, `gcrt1.o`, `grcrt1.o`, `crti.o`, `crtn.o`, `Mcrt1.o`), the `libc.so` linker script, `libc_nonshared.a`, the static archives (`libc.a`, `libm.a`, and the empty `libpthread.a`, `libdl.a`, `librt.a`, `libutil.a`, `libanl.a`, `libresolv.a`, `libg.a`, `libmcheck.a`, `libpthread_nonshared.a`, and `libBrokenLocale.a` with the library's override), the `libm.so` link, the development links `libresolv.so`, `libBrokenLocale.so`, `libanl.so`, `libc_malloc_debug.so`, and the `niji-gcc` / `niji-g++` compiler wrapper |

Everything is inside the sysroot `/usr/lib/niji`, laid out as Debian lays out `/` (merged
`/usr`, multiarch): the libraries in `/usr/lib/niji/usr/lib/x86_64-linux-gnu/`, the loader also
at `/usr/lib/niji/lib64/ld-linux-x86-64.so.2`. The only files outside it are
`/usr/bin/niji-gcc`, `/usr/bin/niji-g++`, their manual page and `/usr/share/doc/niji6*`.
The packages have no maintainer scripts: installing them does not touch `/etc/ld.so.conf`, the
system's loader cache or any other file.

Versions of snapshots are `0.0~snapshotYYYYMMDD+<commit>` (the commit's date and its first 12 hex
digits).

## Installing

```sh
sudo apt install ./niji6_<version>_amd64.deb ./niji6-dev_<version>_amd64.deb
```

To verify the downloads, run `sha256sum -c SHA256SUMS` in the directory that holds them: GitHub does not
allow `~` in asset names, so the published names use `.` where the Debian version has `~`
(`niji6_0.0.snapshot...` for `0.0~snapshot...`), and `SHA256SUMS` lists the published names.

`niji6-dev` depends on `gcc`, `binutils` and `libc6-dev`: libniji has no C header set of its own
yet, so programs compile with the system's headers (libniji implements the glibc 2.39 ABI those
headers describe). `g++` is recommended for `niji-g++`.

## Building a program against libniji

```sh
niji-gcc -O2 hello.c -o hello          # dynamic: runs on libniji's loader
niji-gcc -O2 -static hello.c -o hello  # static: libniji's libc.a
niji-g++ -O2 prog.cc -o prog           # C++: libstdc++ and the unwinder linked statically
niji-gcc -shared -fPIC plugin.c -o plugin.so
```

A program selects libniji through its program interpreter: `niji-gcc` links with
`--dynamic-linker=/usr/lib/niji/lib64/ld-linux-x86-64.so.2`, libniji's start files and libniji's
`libc.so`, so `readelf -l hello` shows that interpreter and the program never starts on the
system's loader. libniji's loader resolves its own libraries (`libc.so.6`, `libm.so.6`, ...) only in
`/usr/lib/niji/usr/lib/x86_64-linux-gnu`, and its default search for other libraries is that
directory too (its cache would be `/usr/lib/niji/etc/ld.so.cache`; none is installed). A program
that needs a system library (`-lz`, ...) names its directory, for example
`-Wl,-rpath,/usr/lib/x86_64-linux-gnu`; libniji's own libraries still come from the sysroot.
`-static-pie` and `-pg` are not supported by the wrapper. See `man niji-gcc`.

The loader finds a program's `DT_NEEDED` libraries named by soname at start-up with the same
search `ld.so --list` and `dlopen` use (`DT_RPATH`, `LD_LIBRARY_PATH`, `DT_RUNPATH`, the cache,
then the default directories); a library that cannot be found ends the start with
`<program>: error while loading shared libraries: <name>: cannot open shared object file: No such
file or directory`, status 127.

Preloading works as ld.so(8) describes: the objects named in `LD_PRELOAD` (separated by spaces or
colons), then those in the preload file `<prefix>/etc/ld.so.preload` (`/usr/lib/niji/etc/ld.so.preload`
for the coexisting packages, `/etc/ld.so.preload` for the system ones), load after the program and
before its libraries, so their symbols interpose; `fakeroot` works this way. A name without a slash is
searched like a `DT_NEEDED` name. An object that cannot be preloaded prints `ERROR: ld.so: object
'<name>' from LD_PRELOAD cannot be preloaded (<reason>): ignored.` and the program runs. A set-user-ID
or set-group-ID program takes an `LD_PRELOAD` name only without a slash, only from the trusted
directories and only when the library is set-user-ID (spec/behaviour/dynlink-preload.md,
spec/behaviour/dynlink-secure.md).

### Registering exit and fork handlers without libc_nonshared.a (opt-in)

By default a module takes `atexit`, `at_quick_exit` and `pthread_atfork` from `libc_nonshared.a`,
as with glibc, and the result also runs on glibc. Nothing about that default is deprecated. A
module may opt in to registering through `libc.so.6` itself: link it with the link-stub directory
`niji-owner/` of the build products (`target/niji/niji-owner/`) ahead of the products
directory, for example `-L target/niji/niji-owner -L target/niji -lc`. The module then needs
the version `NIJI_OWNER_1` and runs only on libniji; glibc's loader refuses it with
"version `NIJI_OWNER_1' not found". libniji attributes each such registration to the calling
module, and runs or drops its handlers before the module is unmapped by `dlclose`
(decision D-27.29).

Migrating a distribution, step by step: build libniji with `NIJI_NONSHARED_NOTICE=1 cargo xtask so`
(the archive's registration members then make the link editor warn, and mark the linked module),
run programs with `NIJI_LD_REPORT_NONSHARED=1` to have libniji's loader name each loaded module
that still registers through the archive, and relink those modules with `niji-owner/` once
two-way compatibility with glibc is no longer needed for them. Both settings are off by default.

## Building the packages

```sh
tools/deb/build.sh            # from a git tree: exports HEAD, builds, writes target/deb/*.deb
tools/deb/check.sh target/deb # paths, no maintainer scripts, the wrapper's interpreter, static smoke
tools/deb/proof.sh target/deb trixie      # unprivileged chroot (mmdebstrap --mode=unshare)
tools/deb/proof.sh target/deb excalibur   # needs Devuan's keyring (KEYRING=...)
```

`proof.sh` takes the suite from its official mirror (`deb.debian.org`, `deb.devuan.org/merged`);
`MIRROR=<url>` names a mirror of that archive instead. Trust comes from the keyring, not the
transport: apt accepts only an index signed by a key in `KEYRING`, so a plain-HTTP mirror is fine.
The packages' `Maintainer` is `Liella Maintainers <liella@koiren.ai>`; `DEB_MAINTAINER` overrides
it for a rebuild.

`build.sh` builds the release profile with the packages' loader layout (`NIJI_PREFIX=/usr/lib/niji`,
`NIJI_LIBDIR=usr/lib/x86_64-linux-gnu`); it needs `cargo`, `dpkg-deb` and `strip`. The build is
reproducible: building the same commit twice gives byte-identical packages (times from
`SOURCE_DATE_EPOCH`, root:root owners, fixed modes, sorted members). The design is decision
D-X.94 (`records/decisions/D-X/D-X.94.toml`).

## The system flavour (Yohane Linux)

Built from the same tree, `tools/deb/build.sh --flavour system` makes the packages that install libniji as
the system C library of Yohane Linux, under Debian's binary package names (owner ruling R1) with `Source: niji`.
Do not install them on a Debian or Devuan system that keeps glibc: they replace `libc6`.

| Package | Contents |
|---|---|
| `libc6` | the loader `/usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2` (`/usr/lib64/ld-linux-x86-64.so.2` links to it; the program interpreter is `/lib64/ld-linux-x86-64.so.2`, reached through base-files' `/lib64 -> usr/lib64`), the shared libraries as in `niji6`, the conffile `/etc/ld.so.conf.d/x86_64-linux-gnu.conf`; activates the `ldconfig` trigger |
| `libc-bin` | `Essential: yes`. Conffiles `/etc/ld.so.conf`, `/etc/ld.so.conf.d/libc.conf`, `/etc/gai.conf`; the template `/usr/share/libc-bin/nsswitch.conf`, which its postinst copies to `/etc/nsswitch.conf` when there is none. `/usr/sbin/ldconfig` (static), `/usr/bin/getconf` (static), `/usr/bin/getent` (dynamic, through the system loader) and `/usr/bin/ldd` (static). Handles the `ldconfig` trigger by running `ldconfig`; purging removes `/etc/ld.so.cache` |
| `libc6-dev` | `Provides: libc-dev (= <version>)`; `Depends` also on `linux-libc-dev`, `libcrypt-dev` and `rpcsvc-proto` (kernel headers, `<crypt.h>`, `rpcgen`), as builds expect of `libc6-dev`. Start files (as in `niji6-dev`, `crti.o` and `crtn.o` included), the `libc.so` script, `libc_nonshared.a`, the static archives (with `libBrokenLocale.a` and the empty `libg.a`, `libmcheck.a`, `libpthread_nonshared.a`), `libm.so`, `libresolv.so`, the development links `libanl.so`, `libBrokenLocale.so`, `libc_malloc_debug.so` (`libnss_compat.so`, `libnss_hesiod.so` wait for their libraries). No headers yet: plain `gcc` compiles with the system's glibc headers |
| `libc-dev-bin`, `locales`, `libc-l10n` | placeholders (documentation only) until their programs and data exist (`locales` and `libc-l10n`: `Priority: standard`; `libc-l10n`: `Multi-Arch: foreign`) |
| `libc6-dbg` | `Section: debug`, `Depends: libc6 (= version)`, `Provides: libc-dbg, libc6-dbgsym`. The detached debugging information (symbol table and line tables) of every shared object of `libc6` and every program of `libc-bin`, as `/usr/lib/debug/.build-id/xx/yyyy.debug` named by each file's GNU build-id: with it installed, gdb shows the source file and line of a libniji function. The packaged objects stay stripped |

Versions are `<abi>+niji<YYYYMMDD>.<n>-<rev>` (owner ruling R2), for example `2.39+niji20261114.1-1`:
no epoch, so they sort above glibc 2.39 and below 2.40, and a binary that needs a later ABI is not
installed against them. `NIJI_DEB_SERIAL` sets `<n>` and `NIJI_DEB_REVISION` sets `<rev>` (default 1).

```sh
tools/deb/build.sh --flavour system          # prefix /, writes target/deb-system/*.deb
tools/deb/check-system.sh target/deb-system   # names, fields, layout, conffiles, triggers, prefix, link
tools/deb/root-system.sh target/deb-system    # dpkg --root into an empty root (unshare -r); dynamic hello,
                                              # threads, except, iostream run inside
tools/deb/swap-system.sh target/deb-system trixie  # libc6 + libc-bin swapped into a trixie root; the
                                              # distribution's ls, sort, grep, sed, awk, bash, perl, python3,
                                              # find, fakeroot compared with a stock root (MIRROR, KEYRING)
tools/deb/lint.sh target/deb-system excalibur # lintian in an unprivileged chroot (MIRROR, KEYRING)
```

The products are built with `NIJI_PREFIX=/` and `NIJI_LIBDIR=usr/lib/x86_64-linux-gnu`: libniji's
own sonames resolve to `/usr/lib/x86_64-linux-gnu`, the cache is `/etc/ld.so.cache`, and the default
directories are the system ones. The design is decision D-X.106 (`records/decisions/D-X/D-X.106.toml`).
