# Sourced (never run) at the top of a script that starts root or user-namespace processes, before it
# starts anything: the script re-runs contained, so that nothing it starts outlives it, or its caller,
# when it is killed (SIGKILL included) or ends (docs/testing/harness-containment.md).
#
#   contained_kind=root|user contained_tree=<the tree's root> [CONTAINED_KEEP="NAME ..."]
#   . "$contained_tree/tools/contained.sh"
#
# The re-run gets the script's path ($0), its arguments and its working directory; NIJI_CONTAINED is set
# in it (to the kind), and there this file does nothing. NIJI_CONTAINED=none runs the script uncontained
# (by the caller's choice only: nothing here falls back to it).
#
# root: the script re-runs through crates/core/tests/oracle/contain.sh: as the caller, as process 1 of a
#   PID namespace made by root, so the root processes it starts (`sudo -n ...`) are in that namespace
#   and end with it (pid_namespaces(7)). The tie to this shell is contain.sh's standard input, a FIFO
#   whose write end only this shell holds (opened here, its name removed at once, closed for the
#   re-run): when this shell goes, killed or not, contain.sh's root watcher ends the namespace. Needs
#   `sudo -n` (contain.sh fails without it). The re-run gets PATH, HOME and the variables CONTAINED_KEEP
#   names (contain.sh clears the rest), and no standard input.
# user: the script re-runs (exec, so it keeps its process) as process 1 of a PID namespace inside a
#   user namespace that maps only the caller's ids: `setpriv --pdeathsig KILL -- unshare --user
#   --map-current-user --pid --fork --kill-child --mount-proc`. Its own `unshare -r` nests. Killing it
#   kills `unshare`, whose `--kill-child` ends the namespace; the caller's death does the same through
#   the parent-death signal. The environment and standard streams pass through.
#
# The script is process 1 of its namespace: a default-action signal that a process of the namespace
# sends it is ignored (pid_namespaces(7)); a fault or a signal from outside still ends it.
#
# rootopt: root where `sudo -n` works; elsewhere the script runs uncontained and says so on stderr (for
#   the unprivileged mmdebstrap tools, which also run on hosts without sudo: see the doc).
if [ "$contained_kind" = rootopt ]; then
    contained_kind=root
    if [ -z "${NIJI_CONTAINED:-}" ] && ! sudo -n true 2>/dev/null; then
        echo "$0: no sudo -n here: running uncontained (a killed run can leave its namespace's processes behind)" >&2
        NIJI_CONTAINED=none
    fi
fi
case ${NIJI_CONTAINED:-}:$contained_kind in
root:* | none:* | user:user) ;;
*:root)
    contained_shell=sh
    [ -n "${BASH_VERSION:-}" ] && contained_shell=bash
    contained_d=$(mktemp -d) || exit 125
    mkfifo "$contained_d/hold" || exit 125
    exec 8<>"$contained_d/hold" 9<"$contained_d/hold"
    rm -r "$contained_d"
    set -- "$contained_shell" "$0" "$@"
    for contained_v in ${CONTAINED_KEEP:-}; do
        eval "contained_set=\${$contained_v+x}"
        if [ -n "$contained_set" ]; then eval "set -- \"\$contained_v=\${$contained_v}\" \"\$@\""; fi
    done
    contained_st=0
    sh "$contained_tree/crates/core/tests/oracle/contain.sh" env NIJI_CONTAINED=root HOME="$HOME" "$@" <&9 8>&- 9<&- || contained_st=$?
    exit "$contained_st"
    ;;
*:user)
    contained_shell=sh
    [ -n "${BASH_VERSION:-}" ] && contained_shell=bash
    NIJI_CONTAINED=user
    export NIJI_CONTAINED
    exec setpriv --pdeathsig KILL -- unshare --user --map-current-user --pid --fork --kill-child --mount-proc -- "$contained_shell" "$0" "$@"
    ;;
*)
    echo "tools/contained.sh: contained_kind must be root or user (got '$contained_kind')" >&2
    exit 125
    ;;
esac
