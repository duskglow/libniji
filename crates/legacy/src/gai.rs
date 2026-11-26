//! `getaddrinfo` and `getnameinfo` over the switch (spec §16.2; D-16.2), with RFC 6724
//! destination address selection. The C structures are built by the boundary
//! (`niji_boundary::netdb`).
//!
//! Written under the clean-room rules from the spec, RFC 3493, RFC 6724, the `getaddrinfo(3)`,
//! `getnameinfo(3)` and `gai.conf(5)` man pages, `spec/behaviour/netdb.md` and black-box probes
//! (D-16.2).
//!
//! **Hints.** NULL hints mean `AF_UNSPEC`, any type and protocol, and `AI_V4MAPPED |
//! AI_ADDRCONFIG` (the default `getaddrinfo(3)` documents; POSIX says 0). Flags beyond
//! `AI_PASSIVE` .. `AI_NUMERICSERV` (0x7ff, the IDN flags included) are `EAI_BADFLAGS`, as is
//! `AI_CANONNAME` without a node. Families other than `AF_UNSPEC`, `AF_INET` and `AF_INET6` are
//! `EAI_FAMILY`.
//!
//! **Socket types and protocols** ([`PROTOCOLS`]): stream/TCP, datagram/UDP and raw are the
//! defaults; DCCP, UDP-Lite and SCTP (stream and sequenced packet) are chosen by naming their
//! type or protocol. A type no entry has, or a type and protocol no entry pairs, is
//! `EAI_SOCKTYPE`; a protocol no entry has is a raw socket of that protocol. Without a named
//! service, each default entry that matches the hints is returned (else the first entry that
//! matches); a named service returns every non-raw entry that matches the hints and whose
//! protocol has the service. A raw entry takes a service only as one of the defaults (any type
//! and protocol), else `EAI_SERVICE` (observed).
//!
//! **Services.** A port is decimal digits only, 0–65535 (D-16.2: the reference also takes
//! `" 80"`, `"+80"` and `""`, and reads 65536 as 0); any other text is a service name, looked
//! up per protocol in the `services` database, `EAI_SERVICE` when no protocol has it, and
//! `EAI_NONAME` with `AI_NUMERICHOST`'s service counterpart `AI_NUMERICSERV`.
//!
//! **Hosts.** No node: the wildcard addresses with `AI_PASSIVE` (IPv4 first, unsorted), else
//! the loopback addresses. A numeric node: IPv4 in `inet_aton`'s forms, IPv6 with an optional
//! zone (`%` and an interface name or index), allowed on link-local unicast and multicast
//! addresses only; an IPv4 text for `AF_INET6` is `EAI_ADDRFAMILY` unless `AI_V4MAPPED`; an
//! IPv4-mapped IPv6 text for `AF_INET` is its IPv4 address (observed). `AI_NUMERICHOST` with
//! anything else is `EAI_NONAME`. A name is looked up in the `hosts` database: `AF_UNSPEC` asks
//! once for either family, `AF_INET6` with `AI_V4MAPPED` adds the IPv4 addresses as mapped ones
//! when there is no IPv6 address, or always with `AI_ALL`. The canonical name is the host's
//! official name (the text itself for a numeric node), on the first entry only.
//!
//! **`AI_ADDRCONFIG`** (D-16.2, as the reference behaves when probed): a family is configured
//! when an interface, up or down, has an address of it other than `127.0.0.1` or `::1`
//! (link-local addresses count). The flag narrows the request before the node is read: with
//! one family configured, `AF_UNSPEC` asks for that family only and a request for the other is
//! `EAI_NONAME`; with neither, `AF_UNSPEC` is left as it is and `AF_INET` or `AF_INET6` is
//! `EAI_NONAME`. So a host with `::1` and link-local IPv6 resolves `::1` for `AF_INET6`.
//!
//! **Order.** The addresses are sorted by RFC 6724 (`niji::names::sort`) with the policy of
//! `gai.conf`; the source of each destination is the kernel's route to it (a netlink route
//! query, `niji::net::iface::route`, in place of the spec's `connect` and `getsockname`),
//! its prefix length from the interface addresses. Each address gives one entry per socket
//! type, in the order above.
//!
//! **IDNA** (D-16.4, `niji::names::dns::idna`): with `AI_IDN` a node that is not ASCII is read
//! in the character set of the context's locale and converted by UTS #46 ToASCII (a node that
//! cannot be read or converted is `EAI_IDN_ENCODE`; an ASCII node is used as it is, observed);
//! `AI_CANONIDN` gives the canonical name in Unicode, and `NI_IDN` the host name, converted back
//! to the locale's character set (kept in ASCII when it cannot be, observed).
//!
//! **Errors** of the lookup: `EAI_NONAME` (not found), `EAI_NODATA` (no address of the family),
//! `EAI_AGAIN` (also when no DNS server could be reached, observed; `getnameinfo` too, whatever
//! its flags), `EAI_FAIL` (sources unavailable: the reference reports `EAI_SYSTEM` with
//! `errno` `EBUSY`, an artifact), `EAI_MEMORY`, `EAI_SYSTEM` with the error of a database file
//! that cannot be read.

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};

use niji::error::Errno;
use niji::mem::TryVec;
use niji::names::nss::LookupError;
use niji::names::sort::{self, Candidate, Policy, Table, ENTRY_BYTES, INLINE_ENTRIES};
use niji::names::dns::idna;
use niji::names::{db, Family, Ip};
use niji::net::{addr, iface, Text};
use niji_kernel::sys::Sys;
use niji_kernel::uapi::{AF_INET, AF_INET6};
use niji_rt::chunks::{ChunkPages, Chunks};

use crate::netdb_compat as netdb;
use crate::nss::{Ctx, Watched};
use crate::result_slots::Buf;

/// `AI_PASSIVE`.
pub const AI_PASSIVE: i32 = 0x1;
/// `AI_CANONNAME`.
pub const AI_CANONNAME: i32 = 0x2;
/// `AI_NUMERICHOST`.
pub const AI_NUMERICHOST: i32 = 0x4;
/// `AI_V4MAPPED`.
pub const AI_V4MAPPED: i32 = 0x8;
/// `AI_ALL`.
pub const AI_ALL: i32 = 0x10;
/// `AI_ADDRCONFIG`.
pub const AI_ADDRCONFIG: i32 = 0x20;
/// `AI_IDN`.
pub const AI_IDN: i32 = 0x40;
/// `AI_CANONIDN`.
pub const AI_CANONIDN: i32 = 0x80;
/// `AI_NUMERICSERV`.
pub const AI_NUMERICSERV: i32 = 0x400;
/// Every flag `getaddrinfo` knows.
pub const AI_MASK: i32 = 0x7ff;

/// `NI_NUMERICHOST`.
pub const NI_NUMERICHOST: i32 = 1;
/// `NI_NUMERICSERV`.
pub const NI_NUMERICSERV: i32 = 2;
/// `NI_NOFQDN`.
pub const NI_NOFQDN: i32 = 4;
/// `NI_NAMEREQD`.
pub const NI_NAMEREQD: i32 = 8;
/// `NI_DGRAM`.
pub const NI_DGRAM: i32 = 16;
/// `NI_IDN`.
pub const NI_IDN: i32 = 32;
/// Every flag `getnameinfo` knows (the IDN flags included).
pub const NI_MASK: i32 = 0xff;

/// `EAI_BADFLAGS`.
pub const EAI_BADFLAGS: i32 = -1;
/// `EAI_NONAME`.
pub const EAI_NONAME: i32 = -2;
/// `EAI_AGAIN`.
pub const EAI_AGAIN: i32 = -3;
/// `EAI_FAIL`.
pub const EAI_FAIL: i32 = -4;
/// `EAI_NODATA`.
pub const EAI_NODATA: i32 = -5;
/// `EAI_FAMILY`.
pub const EAI_FAMILY: i32 = -6;
/// `EAI_SOCKTYPE`.
pub const EAI_SOCKTYPE: i32 = -7;
/// `EAI_SERVICE`.
pub const EAI_SERVICE: i32 = -8;
/// `EAI_ADDRFAMILY`.
pub const EAI_ADDRFAMILY: i32 = -9;
/// `EAI_MEMORY`.
pub const EAI_MEMORY: i32 = -10;
/// `EAI_SYSTEM`.
pub const EAI_SYSTEM: i32 = -11;
/// `EAI_OVERFLOW`.
pub const EAI_OVERFLOW: i32 = -12;
/// `EAI_IDN_ENCODE`.
pub const EAI_IDN_ENCODE: i32 = -105;

/// Why `getaddrinfo` or `getnameinfo` failed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GaiError {
    /// An `EAI_*` code.
    Code(i32),
    /// `EAI_SYSTEM` with this `errno`.
    System(Errno),
}

impl From<Errno> for GaiError {
    fn from(e: Errno) -> GaiError {
        if e == Errno::ENOMEM { GaiError::Code(EAI_MEMORY) } else { GaiError::System(e) }
    }
}

/// The `EAI_*` outcome of a failed host lookup.
pub fn of_lookup(e: LookupError) -> GaiError {
    match e {
        LookupError::NotFound => GaiError::Code(EAI_NONAME),
        LookupError::NoData => GaiError::Code(EAI_NODATA),
        LookupError::TryAgain | LookupError::Unreachable => GaiError::Code(EAI_AGAIN),
        LookupError::Unavailable => GaiError::Code(EAI_FAIL),
        LookupError::Os(e) => GaiError::from(e),
    }
}

type R<T> = core::result::Result<T, GaiError>;

/// The hints (`struct addrinfo`'s first four fields).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Hints {
    /// `ai_flags`.
    pub flags: i32,
    /// `ai_family`.
    pub family: i32,
    /// `ai_socktype`.
    pub socktype: i32,
    /// `ai_protocol`.
    pub protocol: i32,
}

/// One result.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Entry {
    /// `ai_socktype`.
    pub socktype: i32,
    /// `ai_protocol`.
    pub protocol: i32,
    /// The address (its family is `ai_family`).
    pub ip: Ip,
    /// The port, host order.
    pub port: u16,
    /// The IPv6 scope (`sin6_scope_id`).
    pub scope: u32,
}

/// `getaddrinfo`'s result.
#[derive(Debug)]
pub struct Found {
    /// The entries, in order.
    pub entries: TryVec<Entry>,
    /// The canonical name, for the first entry (`AI_CANONNAME`).
    pub canon: Option<TryVec<u8>>,
    /// The flags in effect (each entry's `ai_flags`).
    pub flags: i32,
}

/// A socket type and protocol `getaddrinfo` can return.
#[derive(Clone, Copy, Debug)]
pub struct Proto {
    /// `SOCK_*`.
    pub socktype: i32,
    /// `IPPROTO_*` (0 for raw: any).
    pub protocol: i32,
    /// The protocol's name in `services`.
    pub name: &'static [u8],
    /// Returned when the hints name no type and no protocol.
    pub default: bool,
}

const SOCK_STREAM: i32 = 1;
const SOCK_DGRAM: i32 = 2;
const SOCK_RAW: i32 = 3;

/// The socket types and protocols, in result order.
pub const PROTOCOLS: [Proto; 7] = [
    Proto { socktype: SOCK_STREAM, protocol: 6, name: b"tcp", default: true },
    Proto { socktype: SOCK_DGRAM, protocol: 17, name: b"udp", default: true },
    Proto { socktype: 6, protocol: 33, name: b"dccp", default: false },
    Proto { socktype: SOCK_DGRAM, protocol: 136, name: b"udplite", default: false },
    Proto { socktype: SOCK_STREAM, protocol: 132, name: b"sctp", default: false },
    Proto { socktype: 5, protocol: 132, name: b"sctp", default: false },
    Proto { socktype: SOCK_RAW, protocol: 0, name: b"raw", default: true },
];

/// What a service resolves to.
enum Service<'a> {
    None,
    Port(u16),
    Name(&'a [u8]),
}

/// The socket types, protocols and ports for the hints and the service.
fn sockets<S: Sys>(ctx: &Ctx<'_, S>, h: &Hints, service: Option<&[u8]>) -> R<TryVec<(i32, i32, u16)>> {
    let (st, pr) = (h.socktype, h.protocol);
    if st != 0 && !PROTOCOLS.iter().any(|p| p.socktype == st) {
        return Err(GaiError::Code(EAI_SOCKTYPE));
    }
    // The entries the type and protocol select; a protocol no entry has is a raw socket of it.
    let mut cands: TryVec<(Proto, bool)> = TryVec::new();
    for p in PROTOCOLS {
        let type_ok = st == 0 || p.socktype == st;
        let proto_ok = if pr == 0 { true } else { p.protocol == pr };
        if type_ok && proto_ok && !(p.socktype == SOCK_RAW && pr != 0) {
            cands.push((p, st == 0 && pr == 0))?;
        }
    }
    if cands.is_empty() && pr != 0 && (st == 0 || st == SOCK_RAW) {
        cands.push((Proto { socktype: SOCK_RAW, protocol: pr, name: b"raw", default: false }, false))?;
    }
    if cands.is_empty() {
        return Err(GaiError::Code(EAI_SOCKTYPE));
    }
    let service = match service {
        None => Service::None,
        Some(s) if s.iter().all(u8::is_ascii_digit) && !s.is_empty() => Service::Port(db::port(s).ok_or(GaiError::Code(EAI_SERVICE))?),
        Some(_) if h.flags & AI_NUMERICSERV != 0 => return Err(GaiError::Code(EAI_NONAME)),
        Some(s) => Service::Name(s),
    };
    let mut out = TryVec::new();
    match service {
        Service::None | Service::Port(_) => {
            let port = if let Service::Port(p) = service { Some(p) } else { None };
            let defaults = cands.iter().any(|(p, _)| p.default);
            for (p, any) in cands.iter() {
                if (defaults && !p.default) || (!defaults && out.len() == 1) {
                    continue;
                }
                if p.socktype == SOCK_RAW && port.is_some() && !any {
                    continue;
                }
                out.push((p.socktype, p.protocol, port.unwrap_or(0)))?;
            }
            if out.is_empty() {
                return Err(GaiError::Code(EAI_SERVICE));
            }
        }
        Service::Name(name) => {
            for (p, _) in cands.iter().filter(|(p, _)| p.socktype != SOCK_RAW) {
                match netdb::serv_by_name(ctx, name, Some(p.name), |s| s.port) {
                    Ok(port) => out.push((p.socktype, p.protocol, port))?,
                    Err(LookupError::Os(e)) if e == Errno::ENOMEM => return Err(GaiError::Code(EAI_MEMORY)),
                    Err(_) => {}
                }
            }
            if out.is_empty() {
                return Err(GaiError::Code(EAI_SERVICE));
            }
        }
    }
    Ok(out)
}

/// An address with its scope.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Addr {
    ip: Ip,
    scope: u32,
}

/// Whether a scope may be given to `a`: link-local unicast and multicast addresses.
fn scoped(a: &[u8; 16]) -> bool {
    matches!(a, [0xfe, b, ..] if b & 0xc0 == 0x80) || a.first() == Some(&0xff)
}

/// A numeric node: its addresses (`Ok(None)`: not numeric).
fn numeric_node<S: Sys>(ctx: &Ctx<'_, S>, node: &[u8], h: &Hints) -> R<Option<Addr>> {
    let (text, zone) = match node.iter().position(|&c| c == b'%') {
        Some(i) => (node.get(..i).unwrap_or(&[]), Some(node.get(i.saturating_add(1)..).unwrap_or(&[]))),
        None => (node, None),
    };
    if let Some(v6) = addr::parse_v6(text) {
        let scope = match zone {
            None => 0,
            Some(z) => {
                if !scoped(&v6) {
                    return Err(GaiError::Code(EAI_NONAME));
                }
                match z.iter().all(u8::is_ascii_digit).then(|| core::str::from_utf8(z).ok()?.parse::<u32>().ok()).flatten() {
                    Some(n) if !z.is_empty() => n,
                    _ => iface::index_of(ctx.sys, z).map_err(|_| GaiError::Code(EAI_NONAME))?,
                }
            }
        };
        return match (h.family, Ip::unmapped(&v6)) {
            (f, Some(v4)) if f == fam(AF_INET) => Ok(Some(Addr { ip: Ip::V4(v4), scope: 0 })),
            (f, _) if f == fam(AF_INET) => Err(GaiError::Code(EAI_ADDRFAMILY)),
            _ => Ok(Some(Addr { ip: Ip::V6(v6), scope })),
        };
    }
    if zone.is_some() {
        return Ok(None);
    }
    match netdb::numeric(node, Family::V4) {
        Some(Ip::V4(v4)) if h.family == fam(AF_INET6) => {
            if h.flags & AI_V4MAPPED != 0 {
                Ok(Some(Addr { ip: Ip::V6(Ip::V4(v4).to_v6()), scope: 0 }))
            } else {
                Err(GaiError::Code(EAI_ADDRFAMILY))
            }
        }
        Some(ip) => Ok(Some(Addr { ip, scope: 0 })),
        None => Ok(None),
    }
}

fn fam(af: u32) -> i32 {
    i32::try_from(af).unwrap_or(0)
}

/// The configured families (IPv4, IPv6) for `AI_ADDRCONFIG` (D-16.2): a family is configured
/// when some interface, up or down, has an address of it other than `127.0.0.1` or `::1`
/// (observed: other `127/8` addresses, link-local addresses and any address on `lo` count).
/// `None` when the interfaces cannot be read (the request is then left as it is).
fn configured<S: Sys>(sys: &S) -> Option<(bool, bool)> {
    let list = iface::addrs(sys).ok()?;
    let (mut v4, mut v6) = (false, false);
    for a in list.iter() {
        let Some(x) = a.own() else { continue };
        if u32::from(a.family) == AF_INET {
            v4 |= x.get(..4) != Some(&[127, 0, 0, 1][..]);
        } else if u32::from(a.family) == AF_INET6 {
            v6 |= x != [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
        }
    }
    Some((v4, v6))
}

/// `AI_ADDRCONFIG` narrows the request before anything is looked up (D-16.2): with one family
/// configured, `AF_UNSPEC` becomes that family and a request for the other is `EAI_NONAME`;
/// with neither, `AF_UNSPEC` is left as it is and `AF_INET` or `AF_INET6` is `EAI_NONAME`.
/// Returns the hints and the families still asked for.
fn addrconfig<S: Sys>(sys: &S, mut h: Hints, v4ok: bool, v6ok: bool) -> R<(Hints, bool, bool)> {
    let Some((c4, c6)) = configured(sys) else { return Ok((h, v4ok, v6ok)) };
    let (n4, n6) = addrconfig_families((c4, c6), (v4ok, v6ok)).ok_or(GaiError::Code(EAI_NONAME))?;
    if (n4, n6) != (v4ok, v6ok) {
        h.family = fam(if n4 { AF_INET } else { AF_INET6 });
    }
    Ok((h, n4, n6))
}

/// The families (IPv4, IPv6) `AI_ADDRCONFIG` leaves asked for, given the configured families and
/// the families asked for (both for `AF_UNSPEC`); `None` is `EAI_NONAME` (see [`addrconfig`]).
pub fn addrconfig_families(configured: (bool, bool), asked: (bool, bool)) -> Option<(bool, bool)> {
    let (c4, c6) = configured;
    let (v4, v6) = asked;
    if v4 && v6 {
        // AF_UNSPEC: the one configured family alone, else unchanged.
        return Some(if c4 != c6 { (c4, c6) } else { asked });
    }
    ((v4 && c4) || (v6 && c6)).then_some(asked)
}

/// A table's inline entries in [`GaiConf`]'s buffer.
const TABLE_INLINE: usize = INLINE_ENTRIES * ENTRY_BYTES;
/// The words of one entry past the inline ones.
const ENTRY_WORDS: usize = ENTRY_BYTES.div_ceil(8);

/// `gai.conf`, read once (or again when it changes, with `reload yes`). Its three tables
/// (precedence, label, scopev4) are held in atomics, read and written under the file's lock: a
/// table's first [`INLINE_ENTRIES`] entries in `inline`, the rest in `more`
/// ([`ENTRY_WORDS`] words each, table after table), so a table has no limit but memory
/// (CONFIG-TABLES-GROWTH). The chunks only grow and never move.
pub struct GaiConf {
    file: Watched,
    /// `reload yes` in the file read last: read before the lock, so it is its own atomic.
    reload: AtomicBool,
    /// Each table's entries.
    lens: [AtomicUsize; 3],
    inline: Buf<{ 3 * TABLE_INLINE }>,
    more: Chunks,
}

impl Default for GaiConf {
    fn default() -> GaiConf {
        GaiConf::new()
    }
}

impl GaiConf {
    /// Never read.
    pub const fn new() -> GaiConf {
        GaiConf { file: Watched::new(), reload: AtomicBool::new(false), lens: [const { AtomicUsize::new(0) }; 3], inline: Buf::new(), more: Chunks::new() }
    }

    /// The policy of the file `path`. `pages`: the memory the long tables live in.
    pub fn policy<S: Sys>(&self, sys: &S, pages: &dyn ChunkPages, path: &[u8]) -> Policy {
        // Whether to look at the file again: only once read with `reload yes`.
        let reload = self.reload.load(Relaxed);
        self.file.with(sys, path, reload, |t| self.store(pages, t.map_or(Policy::DEFAULT, Policy::parse)), || self.load(pages))
    }

    /// Hold `p`. A table past the inline entries whose memory is out is held as the default
    /// one, never in part.
    fn store(&self, pages: &dyn ChunkPages, mut p: Policy) {
        let extra = |t: &Table| t.entries().len().saturating_sub(INLINE_ENTRIES);
        let total = extra(&p.precedence).saturating_add(extra(&p.label)).saturating_add(extra(&p.scopev4));
        if total > 0 && !self.more.reserve_first(pages, total, ENTRY_WORDS) {
            for (t, d) in [(&mut p.precedence, sort::PRECEDENCE), (&mut p.label, sort::LABEL), (&mut p.scopev4, sort::SCOPEV4)] {
                if t.entries().len() > INLINE_ENTRIES {
                    *t = d;
                }
            }
        }
        let mut at = 0usize;
        for ((k, t), len) in [&p.precedence, &p.label, &p.scopev4].into_iter().enumerate().zip(&self.lens) {
            let all = t.entries();
            let (head, tail) = all.split_at_checked(INLINE_ENTRIES).unwrap_or((all, &[]));
            let mut bytes = [0u8; TABLE_INLINE];
            for (chunk, x) in bytes.chunks_exact_mut(ENTRY_BYTES).zip(head) {
                chunk.copy_from_slice(&x.encode());
            }
            self.inline.store_at(k.saturating_mul(TABLE_INLINE), bytes.get(..head.len().saturating_mul(ENTRY_BYTES)).unwrap_or(&[]));
            for x in tail {
                let mut b = [0u8; ENTRY_WORDS * 8];
                for (d, s) in b.iter_mut().zip(x.encode()) {
                    *d = s;
                }
                if let Some(words) = self.more.get(pages, at, ENTRY_WORDS) {
                    for (w, c) in words.iter().zip(b.chunks_exact(8)) {
                        w.store(u64::from_le_bytes(c.try_into().unwrap_or([0; 8])), Relaxed);
                    }
                }
                at = at.saturating_add(1);
            }
            len.store(all.len(), Relaxed);
        }
        self.reload.store(p.reload, Relaxed);
    }

    /// The policy held. A table that cannot be given (its entries past the inline ones need
    /// memory that is out) is the default one.
    fn load(&self, pages: &dyn ChunkPages) -> Policy {
        let mut at = 0usize;
        let mut tables = [sort::PRECEDENCE, sort::LABEL, sort::SCOPEV4].map(|d| (Table::EMPTY, d));
        for ((k, (t, default)), len) in tables.iter_mut().enumerate().zip(&self.lens) {
            let n = len.load(Relaxed);
            let first = at;
            at = at.saturating_add(n.saturating_sub(INLINE_ENTRIES));
            // Only the entries the table has.
            let mut bytes = [0u8; TABLE_INLINE];
            let body = bytes.get_mut(..n.min(INLINE_ENTRIES).saturating_mul(ENTRY_BYTES)).unwrap_or(&mut []);
            self.inline.load_at(k.saturating_mul(TABLE_INLINE), body);
            let mut ok = true;
            for chunk in body.chunks_exact(ENTRY_BYTES) {
                // In place: cannot fail.
                ok &= sort::Entry::decode(chunk).is_some_and(|x| t.push(x).is_ok());
            }
            if n > INLINE_ENTRIES {
                ok &= self.load_more(pages, t, first, n.saturating_sub(INLINE_ENTRIES));
            }
            if ok && !t.entries().is_empty() {
                core::mem::swap(t, default);
            }
        }
        let [(_, precedence), (_, label), (_, scopev4)] = tables;
        Policy { precedence, label, scopev4, reload: self.reload.load(Relaxed) }
    }

    /// The `n` entries from entry `first` of `more`, onto `t`: false when they do not decode
    /// or memory is out.
    #[cold]
    #[inline(never)]
    fn load_more(&self, pages: &dyn ChunkPages, t: &mut Table, first: usize, n: usize) -> bool {
        for i in first..first.saturating_add(n) {
            let Some(words) = self.more.get(pages, i, ENTRY_WORDS) else { return false };
            let mut b = [0u8; ENTRY_WORDS * 8];
            for (c, w) in b.chunks_exact_mut(8).zip(words) {
                c.copy_from_slice(&w.load(Relaxed).to_le_bytes());
            }
            if !sort::Entry::decode(&b).is_some_and(|x| t.push(x).is_ok()) {
                return false;
            }
        }
        true
    }
}

/// The process's `gai.conf`.
pub static GAI_CONF: GaiConf = GaiConf::new();

/// Sort `addrs` by RFC 6724 (see the module documentation).
fn sort_addrs<S: Sys>(ctx: &Ctx<'_, S>, addrs: &mut [Addr]) {
    if addrs.len() < 2 {
        return;
    }
    let policy = GAI_CONF.policy(ctx.sys, ctx.pages, ctx.paths.gai_conf);
    let ifaddrs = iface::addrs(ctx.sys).ok();
    let mut keyed: TryVec<(Addr, Candidate)> = TryVec::new();
    for a in addrs.iter() {
        let dest = a.ip.to_v6();
        let source = iface::route(ctx.sys, a.ip.bytes()).ok().flatten().and_then(|r| {
            let n = a.ip.bytes().len();
            // The kernel's preferred source, else the first address of the family on the
            // outgoing interface.
            let own = ifaddrs.as_ref().and_then(|l| {
                l.iter().find(|x| x.index == r.oif && x.addr_len() == n && r.prefsrc.is_none_or(|p| x.own() == Some(p)))
            });
            let src = r.prefsrc.or_else(|| own.and_then(|x| x.own()))?;
            let ip = if n == 4 { Ip::V4(src.get(..4)?.try_into().ok()?) } else { Ip::V6(src) };
            let len = own.map_or(if n == 4 { 32 } else { 128 }, |x| x.prefix);
            let len = if n == 4 { len.saturating_add(96) } else { len };
            Some((ip.to_v6(), len))
        });
        if keyed.push((*a, Candidate { dest, source })).is_err() {
            return;
        }
    }
    sort::sort(&policy, keyed.as_mut_slice(), |k| k.1);
    for (d, (a, _)) in addrs.iter_mut().zip(keyed.iter()) {
        *d = *a;
    }
}

/// Whether [`getaddrinfo`] with these arguments consults the switch: only to look a node name
/// up in the `hosts` database or a service name up in `services`. Otherwise (no node, an empty
/// or numeric node, `AI_NUMERICHOST`; no service, a port number, `AI_NUMERICSERV`) it uses the
/// context's kernel and files only, so a context without the switch serves
/// ([`Ctx::bare`]).
pub fn uses_switch(node: Option<&[u8]>, service: Option<&[u8]>, hints: Option<Hints>) -> bool {
    let flags = hints.map_or(AI_V4MAPPED | AI_ADDRCONFIG, |h| h.flags);
    let named_service = service.is_some_and(|s| (s.is_empty() || !s.iter().all(u8::is_ascii_digit)) && flags & AI_NUMERICSERV == 0);
    let named_node = node.is_some_and(|n| {
        if n.is_empty() || flags & AI_NUMERICHOST != 0 {
            return false;
        }
        // As `numeric_node`: IPv6 text (with a zone after `%`), else IPv4 text without a zone.
        match n.iter().position(|&c| c == b'%') {
            Some(i) => addr::parse_v6(n.get(..i).unwrap_or(&[])).is_none(),
            None => addr::parse_v6(n).is_none() && netdb::numeric(n, Family::V4).is_none(),
        }
    });
    named_service || named_node
}

/// Whether [`getnameinfo`] with these flags consults the switch (see [`uses_switch`]): only for
/// a host or service text asked for by name.
pub fn ni_uses_switch(flags: i32, want_host: bool, want_serv: bool) -> bool {
    (want_host && flags & NI_NUMERICHOST == 0) || (want_serv && flags & NI_NUMERICSERV == 0)
}

/// `getaddrinfo(node, service, hints)` (see the module documentation).
pub fn getaddrinfo<S: Sys>(ctx: &Ctx<'_, S>, node: Option<&[u8]>, service: Option<&[u8]>, hints: Option<Hints>) -> R<Found> {
    let h = hints.unwrap_or(Hints { flags: AI_V4MAPPED | AI_ADDRCONFIG, ..Hints::default() });
    if h.flags & !AI_MASK != 0 {
        return Err(GaiError::Code(EAI_BADFLAGS));
    }
    if node.is_none() && service.is_none() {
        return Err(GaiError::Code(EAI_NONAME));
    }
    if node.is_none() && h.flags & AI_CANONNAME != 0 {
        return Err(GaiError::Code(EAI_BADFLAGS));
    }
    let (v4ok, v6ok) = match h.family {
        0 => (true, true),
        f if f == fam(AF_INET) => (true, false),
        f if f == fam(AF_INET6) => (false, true),
        _ => return Err(GaiError::Code(EAI_FAMILY)),
    };
    let (h, v4ok, v6ok) = if h.flags & AI_ADDRCONFIG != 0 { addrconfig(ctx.sys, h, v4ok, v6ok)? } else { (h, v4ok, v6ok) };
    let socks = sockets(ctx, &h, service)?;
    let mut addrs: TryVec<Addr> = TryVec::new();
    let mut canon = None;
    let mut passive_wildcard = false;
    match node {
        None => {
            let passive = h.flags & AI_PASSIVE != 0;
            passive_wildcard = passive;
            let (a4, a6) = if passive { ([0u8; 4], [0u8; 16]) } else { ([127, 0, 0, 1], [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]) };
            if v4ok {
                addrs.push(Addr { ip: Ip::V4(a4), scope: 0 })?;
            }
            if v6ok {
                addrs.push(Addr { ip: Ip::V6(a6), scope: 0 })?;
            }
        }
        Some(node) => match numeric_node(ctx, node, &h)? {
            Some(a) => {
                addrs.push(a)?;
                if h.flags & AI_CANONNAME != 0 {
                    canon = Some(TryVec::from_slice(node)?);
                }
            }
            None if h.flags & AI_NUMERICHOST != 0 || node.is_empty() => return Err(GaiError::Code(EAI_NONAME)),
            None => {
                let ascii;
                let node = if h.flags & AI_IDN != 0 && !node.is_ascii() {
                    ascii = idn_to_ascii(node, ctx.locale)?;
                    ascii.as_slice()
                } else {
                    node
                };
                let mapped = h.flags & AI_V4MAPPED != 0 && v6ok && !v4ok;
                let host = match (v4ok, v6ok) {
                    (true, true) => netdb::host_lookup(ctx, node, Family::Any),
                    (true, false) => netdb::host_lookup(ctx, node, Family::V4),
                    _ => netdb::host_lookup(ctx, node, Family::V6),
                };
                let mut name = None;
                let mut first_err = None;
                match host {
                    Ok(hst) => {
                        name = Some(TryVec::from_slice(hst.name())?);
                        for ip in hst.addrs.iter() {
                            addrs.push(Addr { ip: *ip, scope: 0 })?;
                        }
                    }
                    Err(e) => first_err = Some(e),
                }
                let want_v4 = mapped && (h.flags & AI_ALL != 0 || matches!(first_err, Some(LookupError::NotFound | LookupError::NoData)));
                if want_v4 {
                    match netdb::host_lookup(ctx, node, Family::V4) {
                        Ok(hst) => {
                            if name.is_none() {
                                name = Some(TryVec::from_slice(hst.name())?);
                            }
                            for ip in hst.addrs.iter() {
                                addrs.push(Addr { ip: Ip::V6(ip.to_v6()), scope: 0 })?;
                            }
                        }
                        Err(e) => first_err = first_err.or(Some(e)),
                    }
                }
                if addrs.is_empty() {
                    return Err(of_lookup(first_err.unwrap_or(LookupError::NotFound)));
                }
                if h.flags & AI_CANONNAME != 0 {
                    canon = match name {
                        Some(n) if h.flags & AI_CANONIDN != 0 => Some(idn_to_unicode(&n, ctx.locale)?),
                        n => n,
                    };
                }
            }
        },
    }
    if !passive_wildcard {
        sort_addrs(ctx, addrs.as_mut_slice());
    }
    let mut entries = TryVec::new();
    for a in addrs.iter() {
        for &(socktype, protocol, port) in socks.iter() {
            entries.push(Entry { socktype, protocol, ip: a.ip, port, scope: a.scope })?;
        }
    }
    Ok(Found { entries, canon, flags: h.flags })
}

// ------------------------------------------------------------------------------ IDNA

/// UTS #46 ToASCII of a node in the locale's character set.
fn idn_to_ascii(node: &[u8], loc: Option<&niji::locale::Locale>) -> R<TryVec<u8>> {
    let bad = GaiError::Code(EAI_IDN_ENCODE);
    let enc = loc.map_or(niji::text::Encoding::Ascii, niji::text::Encoding::of);
    let mut cps: TryVec<u32> = TryVec::new();
    let mut st = niji::text::MbState::INITIAL;
    let mut at = 0usize;
    while at < node.len() {
        match niji::text::decode(&mut st, node.get(at..).unwrap_or(&[]), enc) {
            niji::text::Decoded::Char(c, used) if used > 0 => {
                cps.push(c)?;
                at = at.saturating_add(used);
            }
            _ => return Err(bad),
        }
    }
    let mut out = TryVec::new();
    match idna::to_ascii(&cps, &mut out) {
        Ok(()) => Ok(out),
        Err(e) if e == Errno::ENOMEM => Err(GaiError::Code(EAI_MEMORY)),
        Err(_) => Err(bad),
    }
}

/// UTS #46 ToUnicode of a name, in the locale's character set; the name itself when it cannot
/// be written in that character set.
pub fn idn_to_unicode(name: &[u8], loc: Option<&niji::locale::Locale>) -> R<TryVec<u8>> {
    let enc = loc.map_or(niji::text::Encoding::Ascii, niji::text::Encoding::of);
    let mut cps: TryVec<u32> = TryVec::new();
    idna::to_unicode(name, &mut cps)?;
    let mut out = TryVec::new();
    for &c in cps.iter() {
        let mut b = [0u8; niji::text::MAX_ENCODED];
        match niji::text::encode(c, enc, &mut b) {
            Some(n) => out.extend_from_slice(b.get(..n).unwrap_or(&[]))?,
            None => return Ok(TryVec::from_slice(name)?),
        }
    }
    Ok(out)
}

// ------------------------------------------------------------------------------ getnameinfo

/// `getnameinfo`'s texts.
#[derive(Debug, Default)]
pub struct Names {
    /// The host text, when asked for.
    pub host: Option<TryVec<u8>>,
    /// The service text, when asked for.
    pub serv: Option<TryVec<u8>>,
}

/// The domain of this host: its node name after the first dot (`NI_NOFQDN`).
fn own_domain<S: Sys>(sys: &S) -> Option<Text<65>> {
    let u = sys.uname().ok()?;
    let end = u.nodename.iter().position(|&c| c == 0).unwrap_or(u.nodename.len());
    let name = u.nodename.get(..end)?;
    let dot = name.iter().position(|&c| c == b'.')?;
    let mut t = Text::new();
    t.push_all(name.get(dot.checked_add(1)?..)?);
    (!t.as_bytes().is_empty()).then_some(t)
}

/// The numeric text of an address: RFC 5952 for IPv6, with `%` and the scope when it has one:
/// the interface's name for a link-local unicast or link-local multicast address when the
/// interface exists, else the number (observed).
fn numeric_text<S: Sys>(sys: &S, ip: Ip, scope: u32) -> Result<TryVec<u8>, GaiError> {
    let mut out = TryVec::new();
    match ip {
        Ip::V4(a) => out.extend_from_slice(addr::format_v4(a).as_bytes())?,
        Ip::V6(a) => {
            out.extend_from_slice(addr::format_v6(a).as_bytes())?;
            if scope != 0 {
                out.push(b'%')?;
                let link = matches!(a, [0xfe, b, ..] if b & 0xc0 == 0x80) || matches!(a, [0xff, s, ..] if s & 0xf == 2);
                match iface::name_of(sys, scope) {
                    Ok(n) if link => out.extend_from_slice(n.as_bytes())?,
                    _ => {
                        let mut t = Text::<12>::new();
                        t.push_dec(u64::from(scope));
                        out.extend_from_slice(t.as_bytes())?;
                    }
                }
            }
        }
    }
    Ok(out)
}

/// `getnameinfo(sa, flags)`: the texts asked for (`want_host`, `want_serv`). The socket
/// address is a `struct sockaddr_in` or `sockaddr_in6` of at least its size; anything else is
/// `EAI_FAMILY` (D-16.2: the reference also answers `AF_UNIX` addresses).
pub fn getnameinfo<S: Sys>(ctx: &Ctx<'_, S>, sa: &[u8], flags: i32, want_host: bool, want_serv: bool) -> R<Names> {
    if flags & !NI_MASK != 0 {
        return Err(GaiError::Code(EAI_BADFLAGS));
    }
    let family = sa.get(..2).map(|b| u16::from_ne_bytes([b.first().copied().unwrap_or(0), b.get(1).copied().unwrap_or(0)]));
    let port = sa.get(2..4).map_or(0, |b| u16::from_be_bytes([b.first().copied().unwrap_or(0), b.get(1).copied().unwrap_or(0)]));
    let (ip, scope) = match family.map(u32::from) {
        Some(AF_INET) if sa.len() >= 16 => (Ip::V4(sa.get(4..8).and_then(|b| b.try_into().ok()).ok_or(GaiError::Code(EAI_FAMILY))?), 0),
        Some(AF_INET6) if sa.len() >= 28 => {
            let a: [u8; 16] = sa.get(8..24).and_then(|b| b.try_into().ok()).ok_or(GaiError::Code(EAI_FAMILY))?;
            let scope = sa.get(24..28).and_then(|b| <[u8; 4]>::try_from(b).ok()).map_or(0, u32::from_ne_bytes);
            (Ip::V6(a), scope)
        }
        _ => return Err(GaiError::Code(EAI_FAMILY)),
    };
    let mut out = Names::default();
    if want_host {
        if flags & NI_NUMERICHOST != 0 {
            if flags & NI_NAMEREQD != 0 {
                return Err(GaiError::Code(EAI_NONAME));
            }
            out.host = Some(numeric_text(ctx.sys, ip, scope)?);
        } else {
            match netdb::host_by_addr(ctx, ip) {
                Ok(h) => {
                    let mut name = h.name();
                    if flags & NI_NOFQDN != 0 {
                        if let (Some(d), Some(dot)) = (own_domain(ctx.sys), name.iter().position(|&c| c == b'.')) {
                            let tail = name.get(dot.saturating_add(1)..).unwrap_or(&[]);
                            if tail.eq_ignore_ascii_case(d.as_bytes()) {
                                name = name.get(..dot).unwrap_or(name);
                            }
                        }
                    }
                    out.host = Some(if flags & NI_IDN != 0 { idn_to_unicode(name, ctx.locale)? } else { TryVec::from_slice(name)? });
                }
                // Observed (`gai_down.c`): with or without NI_NAMEREQD, not the numeric form.
                Err(LookupError::TryAgain | LookupError::Unreachable) => return Err(GaiError::Code(EAI_AGAIN)),
                Err(LookupError::Os(e)) if e == Errno::ENOMEM => return Err(GaiError::Code(EAI_MEMORY)),
                Err(_) if flags & NI_NAMEREQD != 0 => return Err(GaiError::Code(EAI_NONAME)),
                Err(_) => out.host = Some(numeric_text(ctx.sys, ip, scope)?),
            }
        }
    }
    if want_serv {
        let proto: &[u8] = if flags & NI_DGRAM != 0 { b"udp" } else { b"tcp" };
        let name = if flags & NI_NUMERICSERV != 0 { None } else { netdb::serv_by_port(ctx, port, Some(proto), |s| TryVec::from_slice(s.name)).ok() };
        out.serv = Some(match name {
            Some(n) => n?,
            None => {
                let mut t = Text::<8>::new();
                t.push_dec(u64::from(port));
                TryVec::from_slice(t.as_bytes())?
            }
        });
    }
    Ok(out)
}
