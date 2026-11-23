#!/bin/sh
# Run a differential test's run script with libniji's name-service cache on or off (D-16.10), as
# the calling user, in a private mount namespace whose /dev/shm is a fresh tmpfs: the products put
# the user's shared region there, so a test never uses or leaves a region in the system's
# /dev/shm. `off` sets NIJI_NSCACHE=0, which the run scripts pass on to their root phases; `on`
# leaves the environment as it was. Needs `sudo -n` (the namespace) and `setpriv`.
#
# A tree that itself lives under /dev/shm (a `cargo xtask plants` copy in RAM) would vanish under
# the fresh tmpfs, and the run script with it ("cannot open .../catalogs_run.sh"). So each
# top-level directory of /dev/shm holding the working directory or a path argument (`/dev/shm/x/..`
# or `NAME=/dev/shm/x/..`) is bound back into the new /dev/shm, from a descriptor opened on the old
# one before the mount. Only a directory that already exists there is bound back: a path the
# command is to create (aio_products' scratch, /dev/shm/niji-aio-...) belongs to the private
# /dev/shm and goes with it. The shared region is a top-level file: it still starts fresh.
#
# With --hide-system-locales, the system's locale directory /usr/lib/locale (its compiled locales
# and archive) is an empty tmpfs too. setlocale searches that directory after the LOCPATH
# directories, in both C libraries (setlocale(3), LOCPATH), so the locales a host happens to have
# installed decide what a name not in LOCPATH selects: with a `de_DE@euro` directory installed,
# `de_DE.UTF-8@euro` no longer falls back to LOCPATH's `de_DE.UTF-8`. A test whose locales all come
# from LOCPATH hides it to give the same answer on every host. `plain` (the reference's runs)
# changes no variable.
#
# Usage: nscache_mode.sh [--hide-system-locales] on|off|plain COMMAND ARGS...
set -eu
hide=0
if [ "$1" = --hide-system-locales ]; then
	hide=1
	shift
fi
mode=$1
shift
case $mode in
on) set -- env -u NIJI_NSCACHE "$@" ;;
off) set -- env NIJI_NSCACHE=0 "$@" ;;
plain) ;;
*) echo "nscache_mode.sh: on, off or plain, not $mode" >&2; exit 2 ;;
esac
keep=
for p in "$PWD" "$@"; do
	case $p in *=/dev/shm/?*) p=${p#*=} ;; esac
	case $p in
	/dev/shm/?*)
		n=${p#/dev/shm/}
		n=${n%%/*}
		case " $keep " in *" $n "*) ;; *) keep="$keep $n" ;; esac
		;;
	esac
done
exec sudo -n unshare -m env -i PATH="$PATH" sh -c 'u=$1; g=$2; k=$3; h=$4; shift 4; exec 3</dev/shm; mount -t tmpfs -o mode=1777 tmpfs /dev/shm || exit 1; for n in $k; do [ -d "/proc/$$/fd/3/$n" ] || continue; mkdir "/dev/shm/$n" && mount --no-canonicalize --bind "/proc/$$/fd/3/$n" "/dev/shm/$n" || exit 1; done; exec 3<&-; if [ "$h" = 1 ] && [ -d /usr/lib/locale ]; then mount -t tmpfs -o mode=755 tmpfs /usr/lib/locale || exit 1; fi; exec setpriv --reuid="$u" --regid="$g" --init-groups "$@"' sh "$(id -u)" "$(id -g)" "$keep" "$hide" "$@"
