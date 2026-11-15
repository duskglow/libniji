# Installing the reference packages

Liella has reference Debian packages for Debian 13 (trixie) and Devuan 6 (excalibur), amd64.
They install Liella **beside** the system's C library: glibc (`libc6`, `libc6-dev`) and every
existing program are left as they are, and purging the packages leaves the system as it was.
Replacing `libc6` (Liella as the system's C library) is the separate system flavour at the end of
this page, for Aqours.

## What they are

| Package | Like Debian's | Contents |
|---|---|---|
| `liella6` | `libc6` | the loader `ld-linux-x86-64.so.2` and the shared libraries (`libc.so.6`, `libm.so.6`, `libpthread.so.0`, `libdl.so.2`, `librt.so.1`, `libutil.so.1`, `libanl.so.1`, `libnsl.so.1`, `libc_malloc_debug.so.0`) |
| `liella6-dev` | `libc6-dev` | start files (`crt1.o`, `Scrt1.o`, `rcrt1.o`), the `libc.so` linker script, `libc_nonshared.a`, the static archives (`libc.a`, `libm.a`, and the empty `libpthread.a`, `libdl.a`, `librt.a`, `libutil.a`, `libanl.a`, `libresolv.a`), the `libm.so` link, and the `liella-gcc` / `liella-g++` compiler wrapper |

Everything is inside the sysroot `/usr/lib/liella`, laid out as Debian lays out `/` (merged
`/usr`, multiarch): the libraries in `/usr/lib/liella/usr/lib/x86_64-linux-gnu/`, the loader also
at `/usr/lib/liella/lib64/ld-linux-x86-64.so.2`. The only files outside it are
`/usr/bin/liella-gcc`, `/usr/bin/liella-g++`, their manual page and `/usr/share/doc/liella6*`.
The packages have no maintainer scripts: installing them does not touch `/etc/ld.so.conf`, the
system's loader cache or any other file.

Versions of snapshots are `0.0~snapshotYYYYMMDD+<commit>` (the commit's date and its first 12 hex
digits).

## Installing

```sh
sudo apt install ./liella6_<version>_amd64.deb ./liella6-dev_<version>_amd64.deb
```

`liella6-dev` depends on `gcc`, `binutils` and `libc6-dev`: Liella has no C header set of its own
yet, so programs compile with the system's headers (Liella implements the glibc 2.39 ABI those
headers describe). `g++` is recommended for `liella-g++`.

## Building a program against Liella

```sh
liella-gcc -O2 hello.c -o hello          # dynamic: runs on Liella's loader
liella-gcc -O2 -static hello.c -o hello  # static: Liella's libc.a
liella-g++ -O2 prog.cc -o prog           # C++: libstdc++ and the unwinder linked statically
liella-gcc -shared -fPIC plugin.c -o plugin.so
```

A program selects Liella through its program interpreter: `liella-gcc` links with
`--dynamic-linker=/usr/lib/liella/lib64/ld-linux-x86-64.so.2`, Liella's start files and Liella's
`libc.so`, so `readelf -l hello` shows that interpreter and the program never starts on the
system's loader. Liella's loader resolves its own libraries (`libc.so.6`, `libm.so.6`, ...) only in
`/usr/lib/liella/usr/lib/x86_64-linux-gnu`, and its default search for other libraries is that
directory too (its cache would be `/usr/lib/liella/etc/ld.so.cache`; none is installed). A program
that needs a system library (`-lz`, ...) names its directory, for example
`-Wl,-rpath,/usr/lib/x86_64-linux-gnu`; Liella's own libraries still come from the sysroot.
`-static-pie` and `-pg` are not supported by the wrapper. See `man liella-gcc`.

**Today (2026-11-13):** the loader does not yet search for a program's `DT_NEEDED` libraries at
start-up (it loads them only by path), so dynamically linked programs built with `liella-gcc`
stop with "the library search is not available in this build". Static programs run. Dynamic
programs run once the loader's start-time search lands; nothing in the packages changes then.

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

`build.sh` builds the release profile with the packages' loader layout (`LIELLA_PREFIX=/usr/lib/liella`,
`LIELLA_LIBDIR=usr/lib/x86_64-linux-gnu`); it needs `cargo`, `dpkg-deb` and `strip`. The build is
reproducible: building the same commit twice gives byte-identical packages (times from
`SOURCE_DATE_EPOCH`, root:root owners, fixed modes, sorted members). The design is the pending
decision `records/pending/RELEASE-PACKAGING-DEB-1.toml`.

## The system flavour (Aqours)

Built from the same tree, `tools/deb/build.sh --flavour system` makes the packages that install Liella as
the system C library of Aqours, under Debian's binary package names (owner ruling R1) with `Source: liella`.
Do not install them on a Debian or Devuan system that keeps glibc: they replace `libc6`.

| Package | Contents |
|---|---|
| `libc6` | the loader `/usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2` (`/usr/lib64/ld-linux-x86-64.so.2` links to it; the program interpreter is `/lib64/ld-linux-x86-64.so.2`, reached through base-files' `/lib64 -> usr/lib64`), the shared libraries as in `liella6`, the conffile `/etc/ld.so.conf.d/x86_64-linux-gnu.conf`; activates the `ldconfig` trigger |
| `libc-bin` | `Essential: yes`. Conffiles `/etc/ld.so.conf`, `/etc/ld.so.conf.d/libc.conf`, `/etc/gai.conf`; the template `/usr/share/libc-bin/nsswitch.conf`, which its postinst copies to `/etc/nsswitch.conf` when there is none. Handles the `ldconfig` trigger: a no-op until Liella ships `ldconfig` (the loader searches the standard directories without a cache) |
| `libc6-dev` | `Provides: libc-dev`. Start files, the `libc.so` script, `libc_nonshared.a`, the static archives, `libm.so`. No headers and no `crti.o`/`crtn.o` yet: plain `gcc` links only with `-nostartfiles` and the system's glibc headers |
| `libc-dev-bin`, `locales`, `libc-l10n` | placeholders (documentation only) until their programs and data exist |

Versions are `<abi>+liella<YYYYMMDD>.<n>-<rev>` (owner ruling R2), for example `2.39+liella20261114.1-1`:
no epoch, so they sort above glibc 2.39 and below 2.40, and a binary that needs a later ABI is not
installed against them. `LIELLA_DEB_SERIAL` sets `<n>` and `LIELLA_DEB_REVISION` sets `<rev>` (default 1).

```sh
tools/deb/build.sh --flavour system          # prefix /, writes target/deb-system/*.deb
tools/deb/check-system.sh target/deb-system   # names, fields, layout, conffiles, triggers, prefix, link
tools/deb/root-system.sh target/deb-system    # dpkg --root into an empty root (unshare -r), hello inside
tools/deb/lint.sh target/deb-system excalibur # lintian in an unprivileged chroot (MIRROR, KEYRING)
```

The products are built with `LIELLA_PREFIX=/` and `LIELLA_LIBDIR=usr/lib/x86_64-linux-gnu`: Liella's
own sonames resolve to `/usr/lib/x86_64-linux-gnu`, the cache is `/etc/ld.so.cache`, and the default
directories are the system ones. The design is the pending decision `records/pending/SL-FLAVOUR-1.toml`.
