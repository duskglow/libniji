#!/bin/sh
# Run a command so that nothing it starts can outlive it or its caller (dns_products.rs). The
# oracle scripts start root processes (`sudo -n unshare -m -n` and the probes' DNS servers under
# it) that a user-level `timeout` cannot end; a run that hung or was killed used to leave them
# behind. Here the command runs, as the calling user again, as the first process of its own PID
# namespace: when that process ends, or is killed, the kernel kills every process in the namespace,
# root or not, however deep (pid_namespaces(7)); it has its own /proc. The namespace's first
# process is in turn killed when the caller goes: standard input must be a pipe whose write end
# only the caller holds (close-on-exec); its end (the caller exited or was killed) kills `unshare`,
# whose parent-death signal (`--kill-child`) takes the namespace with it. Changing back to the
# user clears that signal (PR_SET_PDEATHSIG, prctl(2)), so a second `setpriv` sets it again.
#
# Usage: contain.sh COMMAND [ARG...]     (stdin: the caller's pipe; needs `sudo -n`)
# COMMAND gets the caller's PATH and no other environment (sudo would have replaced it).
# Exit status: COMMAND's; 137 when the caller went first.
set -eu
exec sudo -n sh -c '
    uid=$1 gid=$2 path=$3
    shift 3
    exec 3<&0
    unshare --pid --fork --kill-child --mount-proc -- setpriv --reuid="$uid" --regid="$gid" --init-groups -- \
        setpriv --pdeathsig KILL -- env -i PATH="$path" "$@" </dev/null 3<&- &
    ns=$!
    (read -r _ <&3 || :; kill -KILL "$ns" 2>/dev/null) &
    watch=$!
    set +e
    wait "$ns"
    status=$?
    kill "$watch" 2>/dev/null
    exit "$status"
' sh "$(id -u)" "$(id -g)" "$PATH" "$@"
