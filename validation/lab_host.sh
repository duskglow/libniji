# LAB-ONLY: sourced by the scripts that drive the project's own test hosts; not needed to build or test libniji.
# lab_addr <host name>: print the ssh destination (address or name) for a lab host. Addresses are never stored in
# the repository (static check `home-addr`, D-X.45); they come from the operator's machine, first match wins:
#   1. NIJI_HOST_<NAME> (upper case), e.g. NIJI_HOST_KEKE=192.0.2.10
#   2. the map file ${NIJI_LAB_HOSTS:-$HOME/.config/niji/lab-hosts}: one `name address` pair per line, # comments
#   3. the ops checkout's vm_ssh.sh (${NIJI_OPS:-$HOME/src/liella-ops}/tools/vm_ssh.sh): its ssh destination
#   4. the bare name (works when DNS or ~/.ssh/config resolves it)
lab_addr() {
    local name=$1 var v map ops
    var="NIJI_HOST_$(printf '%s' "$name" | tr '[:lower:]' '[:upper:]')"
    v=${!var:-}
    if [ -n "$v" ]; then printf '%s\n' "$v"; return 0; fi
    map=${NIJI_LAB_HOSTS:-$HOME/.config/niji/lab-hosts}
    if [ -r "$map" ]; then
        v=$(awk -v n="$name" '$1 == n && $1 !~ /^#/ { print $2; exit }' "$map")
        if [ -n "$v" ]; then printf '%s\n' "$v"; return 0; fi
    fi
    ops=${NIJI_OPS:-$HOME/src/liella-ops}/tools/vm_ssh.sh
    if [ -r "$ops" ]; then
        v=$( (. "$ops" && vm_ssh_cmd "$name" >/dev/null 2>&1 && printf '%s\n' "${VSSH[${#VSSH[@]}-1]}") 2>/dev/null)
        v=${v##*@}
        if [ -n "$v" ]; then printf '%s\n' "$v"; return 0; fi
    fi
    printf '%s\n' "$name"
}
