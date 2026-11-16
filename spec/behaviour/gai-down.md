# Name lookups when no DNS server answers (getaddrinfo rows; GitHub issue #8)

Observed black-box on the reference (glibc 2.39, Zen 1 host, 2026-11-16) with
`crates/core/tests/oracle/gai_down.c` run by `gai_down_run.sh`: private user, mount and network
namespaces, only `lo` up, a fixture `/etc` bind-mounted (`services` always present; resolv.conf
`options timeout:1 attempts:1`). Checked line by line against libniji by
`crates/core/tests/gai_down_products.rs`. Spec: getaddrinfo(3), RFC 1035; §16.1, §16.2, §16.4
(clauses s16_2_f_0, s16_2_c_1, s16_2_c_4, s16_1_c_2, s16_4_c_0).

Scenarios (server on 127.0.0.1:53 where one runs):

| scenario | resolv.conf | nsswitch hosts | /etc/hosts | server |
|---|---|---|---|---|
| unreach | nameserver 192.0.2.1 (no route: ENETUNREACH) | files dns | yes | none |
| unreach_nohosts / unreach_bare | same | files dns / no nsswitch.conf | no | none |
| unreach_v6 | nameserver 2001:db8::1 | files dns | yes | none |
| refused / refused_nohosts | nameserver 127.0.0.1 (ECONNREFUSED) | files dns | yes / no | none |
| refused_unavail | nameserver 127.0.0.1 | dns [!UNAVAIL=return] files | yes | none |
| two_down | 192.0.2.1 and 127.0.0.1 | files dns | yes | none |
| timeout / timeout_nohosts | nameserver 127.0.0.1 | files dns | yes / no | never answers |
| servfail | nameserver 127.0.0.1 | files dns | yes | SERVFAIL |
| trunc_notcp | nameserver 127.0.0.1 | files dns | yes | TC set, no TCP listener |
| empty_resolv / no_resolv / no_resolv_nohosts | empty / none / none | files dns | yes / yes / no | none |
| recover, recover_unavail | nameserver 127.0.0.1 | files dns / dns [!UNAVAIL=return] files | yes | none, then answering |

Results, every scenario where DNS gives no answer (a name not in `/etc/hosts`, or any name
with no `/etc/hosts`):

| call | result | errno | h_errno |
|---|---|---|---|
| `getaddrinfo(name, "http", NULL)` and with hints | -3 `EAI_AGAIN` | not reported | not set |
| `gethostbyname`, `gethostbyname2(AF_INET6)` | NULL | `EAGAIN` | 2 `TRY_AGAIN` |
| `gethostbyname_r` (4096-byte buffer) | returns `EAGAIN`, `*result` NULL | `EAGAIN` | `*h_errnop` 2 `TRY_AGAIN` |
| `gethostbyaddr(192.0.2.10)` | NULL | unchanged (0) | 2 `TRY_AGAIN` |
| `getnameinfo(192.0.2.10:80, flags 0)` | -3 `EAI_AGAIN` (not the numeric form) | - | - |
| `getnameinfo(..., NI_NAMEREQD)` | -3 `EAI_AGAIN` | - | - |

* A missing `/etc/hosts` or `nsswitch.conf` does not change these (no `EAI_SYSTEM`); `localhost`
  then gives `EAI_AGAIN` too (the issue's third row). With `/etc/hosts`, its names are found.
* An empty or missing `resolv.conf` asks 127.0.0.1 (refused): the same results.
* `refused_unavail`: a refused name lookup is `unavail` for the rules, so `files` runs: unknown
  names are -2 `EAI_NONAME`, `gethostbyname` `HOST_NOT_FOUND` (errno 0). The address lookup is
  not: `gethostbyaddr` `TRY_AGAIN`, `getnameinfo` `EAI_AGAIN`.
* `recover`: with the server up in the same process, every lookup gets the server's answer
  (nothing failed earlier is kept); `recover_unavail` likewise (the `files` answers given while the
  server was down are not served afterwards: `localhost` is then NXDOMAIN, `EAI_NONAME`).

libniji: the `dns` source reports a lookup that reached no server as `LookupError::Unreachable`
(`unavail` for the rules; `TRY_AGAIN` / `EAI_AGAIN` for the caller), an address lookup without a
response as `TryAgain`, and the name-service cache (D-16.10) keeps nothing found by a lookup
that got no DNS response.
