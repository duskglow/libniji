# SOCK-NULL-OPTVAL: NULL socket option and name buffers reach the kernel (GitHub issue #17)

State: branch `task/sock-null-optval` (from origin/next e40107432). The code is in commit fab42ebae; the
record and the test rename follow it. Gate and plants ran on keke `~/w/n4`.

## What changed

- `getsockopt` passes the caller's `optval`/`optlen` addresses to the kernel as given (`raw::getsockopt`, new).
  The typed path (`net_compat::getsockopt`, removed) turned a NULL value into `EFAULT` before the call (the
  issue). It also lost the length the kernel stores on `ERANGE` (`SO_PEERGROUPS`, `SO_PEERSEC`) and
  mapped a negative level to `EINVAL` where the kernel says `EOPNOTSUPP`.
- `getsockname`/`getpeername`: kernel call first, then the caller's buffer (`conv::name_out`, new;
  `net_compat::sockname` takes the buffer as a closure). NULL with `*len` 0 is a length query.
- `setsockopt` with a NULL value goes to the kernel through `raw::setsockopt`.
- Decision: `records/pending/SOCK-NULL-OPTVAL-1.toml` (D-NEW, series D-15, amends D-15.5); CI assigns the id.

## Step 2 table (black-box probe, reference / libniji before / after; r, errno, *len)

The before column was measured with the probe on next. The after column was expected to equal the reference
(where they differ, after = reference, which the kernel decides). The tests check it, but the probe was not
re-run on the head.

| case | reference | before (next) | after |
|---|---|---|---|
| getsockopt netlink LIST_MEMBERSHIPS NULL, len 0 (no group) | 0, len 0 | -1 EFAULT | = reference |
| same, one group joined | 0, len 8 | -1 EFAULT | = reference |
| same, NULL, len 4 | -1 EFAULT, len 8 | -1 EFAULT, len 4 | = reference |
| getsockopt SO_TYPE NULL, len 0 | 0, len 0 | -1 EFAULT | = reference |
| getsockopt(-1, NULL, NULL) | EBADF | EFAULT | = reference |
| getsockopt level -5 | EOPNOTSUPP | EINVAL | = reference |
| getsockopt SO_PEERGROUPS NULL, len 0 | ERANGE, len 12 | EFAULT, len 0 | = reference |
| getsockopt SO_PEERGROUPS 1-byte buffer | ERANGE, len 12 | ERANGE, len 1 | = reference |
| getsockopt SO_PEERSEC NULL, len 0 | ERANGE, len 10 | EFAULT | = reference |
| getsockname NULL, len 0 | 0, len 16 | EFAULT | = reference |
| getsockname(-1, NULL, NULL) | EBADF | EFAULT | = reference |
| getsockname NULL, len 2^31 | EINVAL | EFAULT | = reference |
| getpeername unconnected NULL, len 0 | ENOTCONN | EFAULT | = reference |
| getpeername socketpair NULL, len 0 | 0, len 2 | EFAULT | = reference |
| setsockopt(-1, SO_REUSEADDR, NULL, 4) | EBADF | EFAULT | = reference |
| same cases, already equal | getsockopt buf/NULL len, len 2^31, name -5; getsockname NULL len 16, buf/NULL len; setsockopt NULL 0, NULL 4, len 2^31, name -5 | same | same |

Not changed: when both the descriptor and the pointer are bad, these still fail with `EFAULT` where the
reference gives `EBADF`. The return value is the same: `accept`/`recvfrom` with an address and no length cell
(the library writes `*len` itself), and a NULL buffer with a non-zero length for `send`/`recv`/`sendto`,
`bind`/`connect`, and a NULL `msghdr` for `sendmsg`/`recvmsg`. The other `conv::req` users outside §15 were
not surveyed.

## Next step

A follow-up story on error order: let the kernel see a NULL buffer with a non-zero length in the slice-based
socket calls (send/recv/bind/connect/msghdr) and survey the remaining `conv::req` users. The probe source is
in the hand-back, not in the tree.

## Not to redo

- In a probe that prints with `strerrorname_np`, define `_GNU_SOURCE`. Without it the call is implicitly
  declared `int`, so the dynamic reference build crashes and the static libniji build does not. That cost
  two probe runs here.
- The installed `~/bin/probe` (keke) is older than `tools/probe/probe` and looks for `target/liella`. Run
  `python3 ~/w/<role>/tools/probe/probe --niji ~/w/<role> ...` instead.
