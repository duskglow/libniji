//! The C synchronisation objects (`niji_rt::pthread_sync`) under real threads: mutual
//! exclusion for every type, the error-checking and recursive contracts, timeouts on both
//! clocks, the condition variable's ordering guarantee (§22.5), read-write lock preferences,
//! barriers over many rounds, once objects, and semaphores.
#![allow(clippy::arithmetic_side_effects, clippy::unwrap_used, clippy::indexing_slicing, clippy::as_conversions, missing_docs)]

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering::SeqCst};
use std::sync::Arc;
use std::time::{Duration, Instant};

use niji_boundary::sys::CSys;
use niji_kernel::errno::{Errno, KResult};
use niji_kernel::sys::Sys;
use niji_kernel::uapi::Timespec;
use niji_rt::futex::{Clock, Scope};
use niji_rt::pthread_sync::{self as ps, Barrier, Cond, Ctx, LockWait, Mutex, RwLock, Sem};

struct T;

impl Ctx for T {
    type S = CSys;
    fn sys(&self) -> &CSys {
        &CSys
    }
    fn tid(&self) -> u32 {
        CSys.gettid() as u32
    }
    fn robust(&self) -> Option<&[AtomicUsize; 3]> {
        None
    }
    fn links(&self, _: usize) -> Option<&[AtomicUsize]> {
        None
    }
}

struct Obj {
    w: Vec<AtomicU32>,
    l: Vec<AtomicUsize>,
}

impl Obj {
    fn new(n: usize) -> Arc<Obj> {
        Arc::new(Obj { w: (0..n).map(|_| AtomicU32::new(0)).collect(), l: (0..2).map(|_| AtomicUsize::new(0)).collect() })
    }
    fn mutex(&self) -> Mutex<'_> {
        Mutex::new(&self.w, &self.l, 0)
    }
}

fn after(clock: Clock, d: Duration) -> Timespec {
    let id = if clock == Clock::Realtime { 0 } else { 1 };
    let now = CSys.clock_gettime(id).unwrap();
    let ns = now.nsec + d.subsec_nanos() as i64;
    Timespec { sec: now.sec + d.as_secs() as i64 + ns / 1_000_000_000, nsec: ns % 1_000_000_000 }
}

#[test]
fn every_mutex_type_excludes() {
    // Clauses: s22_4_c_0, s22_4_c_1 (D-22.1).
    for kind in [ps::MUTEX_NORMAL, ps::MUTEX_RECURSIVE, ps::MUTEX_ERRORCHECK, ps::MUTEX_ADAPTIVE] {
        let m = Obj::new(10);
        m.mutex().init(kind, 0);
        let counter = Arc::new(AtomicU32::new(0));
        let inside = Arc::new(AtomicU32::new(0));
        let hs: Vec<_> = (0..8)
            .map(|_| {
                let (m, counter, inside) = (Arc::clone(&m), Arc::clone(&counter), Arc::clone(&inside));
                std::thread::spawn(move || {
                    for _ in 0..5000 {
                        m.mutex().lock(&T, LockWait::Block).unwrap();
                        assert_eq!(inside.fetch_add(1, SeqCst), 0, "kind {kind}: two holders");
                        counter.fetch_add(1, SeqCst);
                        // MUTEX-MT: whichever path took it (fast or contended), `__count` is
                        // the recursion depth of a recursive mutex and 0 in the other types.
                        assert_eq!(m.w[1].load(SeqCst), u32::from(kind == ps::MUTEX_RECURSIVE), "kind {kind}: count word");
                        inside.fetch_sub(1, SeqCst);
                        m.mutex().unlock(&T).unwrap();
                    }
                })
            })
            .collect();
        for h in hs {
            h.join().unwrap();
        }
        assert_eq!(counter.load(SeqCst), 40_000);
        assert_eq!(m.mutex().destroy(), Ok(()));
    }
}

#[test]
fn only_a_recursive_mutex_keeps_a_count() {
    // Clauses: s22_4_c_0, s22_4_c_1 (MUTEX-MT: black-box probes show `__count` 0 for normal,
    // error-checking and adaptive mutexes, held or not, and the depth for a recursive one).
    for kind in [ps::MUTEX_NORMAL, ps::MUTEX_RECURSIVE, ps::MUTEX_ERRORCHECK, ps::MUTEX_ADAPTIVE] {
        for alone in [true, false] {
            let o = Obj::new(10);
            o.mutex().init(kind, 0);
            let lock = || if alone { o.mutex().lock(&Alone, LockWait::Block) } else { o.mutex().lock(&T, LockWait::Block) };
            let unlock = || if alone { o.mutex().unlock(&Alone) } else { o.mutex().unlock(&T) };
            let rec = kind == ps::MUTEX_RECURSIVE;
            lock().unwrap();
            assert_eq!(o.w[1].load(SeqCst), u32::from(rec), "kind {kind} alone {alone}: held once");
            if rec {
                lock().unwrap();
                assert_eq!(o.w[1].load(SeqCst), 2, "kind {kind} alone {alone}: held twice");
                unlock().unwrap();
                assert_eq!(o.w[1].load(SeqCst), 1, "kind {kind} alone {alone}: held once again");
            }
            unlock().unwrap();
            assert_eq!(o.w[1].load(SeqCst), 0, "kind {kind} alone {alone}: released");
            assert_eq!(o.w[0].load(SeqCst), 0, "kind {kind} alone {alone}: lock word");
        }
    }
}

/// MUTEX-2: the context of a process's only thread (plain loads and stores on the fast paths).
struct Alone;

impl Ctx for Alone {
    type S = CSys;
    fn sys(&self) -> &CSys {
        &CSys
    }
    fn tid(&self) -> u32 {
        CSys.gettid() as u32
    }
    fn alone(&self) -> bool {
        true
    }
    fn robust(&self) -> Option<&[AtomicUsize; 3]> {
        None
    }
    fn links(&self, _: usize) -> Option<&[AtomicUsize]> {
        None
    }
}

/// One mutex operation, through the only thread's context (`true`) or the general one.
type Step<'a> = dyn Fn(&Obj, bool) -> Result<(), Errno> + 'a;

#[test]
fn only_thread_paths_leave_the_words_the_atomic_paths_leave() {
    // Clauses: s22_4_c_0, s22_4_c_1, s22_4_f_0 (MUTEX-2).
    let words = |o: &Obj| o.w.iter().map(|x| x.load(SeqCst)).collect::<Vec<_>>();
    for kind in [ps::MUTEX_NORMAL, ps::MUTEX_RECURSIVE, ps::MUTEX_ERRORCHECK, ps::MUTEX_ADAPTIVE] {
        let (a, b) = (Obj::new(10), Obj::new(10));
        a.mutex().init(kind, 0);
        b.mutex().init(kind, 0);
        // The same sequence through both contexts: lock, try, relock, unlock twice, unlock.
        let steps: [&Step<'_>; 6] = [
            &|o, al| if al { o.mutex().lock(&Alone, LockWait::Block) } else { o.mutex().lock(&T, LockWait::Block) },
            &|o, al| if al { o.mutex().lock(&Alone, LockWait::Try) } else { o.mutex().lock(&T, LockWait::Try) },
            &|o, al| {
                if kind == ps::MUTEX_NORMAL || kind == ps::MUTEX_ADAPTIVE {
                    Ok(()) // relocking would deadlock
                } else if al {
                    o.mutex().lock(&Alone, LockWait::Block)
                } else {
                    o.mutex().lock(&T, LockWait::Block)
                }
            },
            &|o, al| if al { o.mutex().unlock(&Alone) } else { o.mutex().unlock(&T) },
            &|o, al| if al { o.mutex().unlock(&Alone) } else { o.mutex().unlock(&T) },
            &|o, al| if al { o.mutex().unlock(&Alone) } else { o.mutex().unlock(&T) },
        ];
        for (i, s) in steps.iter().enumerate() {
            assert_eq!(s(&a, true), s(&b, false), "kind {kind} step {i}: result");
            assert_eq!(words(&a), words(&b), "kind {kind} step {i}: words");
        }
        assert_eq!(a.mutex().destroy(), Ok(()), "kind {kind}: released");
    }
    // A waiter mark (2) is released through the general path, which wakes.
    let m = Obj::new(10);
    m.mutex().init(ps::MUTEX_NORMAL, 0);
    m.mutex().lock(&Alone, LockWait::Block).unwrap();
    m.w[0].store(2, SeqCst);
    m.mutex().unlock(&Alone).unwrap();
    assert_eq!(m.w[0].load(SeqCst), 0);
}

#[test]
fn errorcheck_and_recursive_contracts() {
    // Clauses: s22_4_c_1, s22_4_f_0.
    let e = Obj::new(10);
    e.mutex().init(ps::MUTEX_ERRORCHECK, 0);
    assert_eq!(e.mutex().unlock(&T), Err(Errno::EPERM), "unlock of an unlocked mutex");
    e.mutex().lock(&T, LockWait::Block).unwrap();
    assert_eq!(e.mutex().lock(&T, LockWait::Block), Err(Errno::EDEADLK));
    assert_eq!(e.mutex().destroy(), Err(Errno::EBUSY));
    let e2 = Arc::clone(&e);
    std::thread::spawn(move || {
        assert_eq!(e2.mutex().unlock(&T), Err(Errno::EPERM), "a non-owner");
        assert_eq!(e2.mutex().lock(&T, LockWait::Try), Err(Errno::EBUSY));
    })
    .join()
    .unwrap();
    e.mutex().unlock(&T).unwrap();

    let r = Obj::new(10);
    r.mutex().init(ps::MUTEX_RECURSIVE, 0);
    for _ in 0..3 {
        r.mutex().lock(&T, LockWait::Block).unwrap();
    }
    r.mutex().unlock(&T).unwrap();
    r.mutex().unlock(&T).unwrap();
    let r2 = Arc::clone(&r);
    std::thread::spawn(move || assert_eq!(r2.mutex().lock(&T, LockWait::Try), Err(Errno::EBUSY), "still held once")).join().unwrap();
    r.mutex().unlock(&T).unwrap();
    assert_eq!(r.mutex().unlock(&T), Err(Errno::EPERM));
    // The recursion count cannot overflow (§22.4: EAGAIN).
    r.mutex().lock(&T, LockWait::Block).unwrap();
    r.w[1].store(u32::MAX, SeqCst);
    assert_eq!(r.mutex().lock(&T, LockWait::Block), Err(Errno::EAGAIN));
    r.w[1].store(1, SeqCst);
    r.mutex().unlock(&T).unwrap();
}

#[test]
fn timed_locks_time_out_on_both_clocks() {
    // Clauses: s22_4_c_5, s22_4_f_0.
    let m = Obj::new(10);
    m.mutex().init(ps::MUTEX_NORMAL, 0);
    m.mutex().lock(&T, LockWait::Block).unwrap();
    let m2 = Arc::clone(&m);
    std::thread::spawn(move || {
        for clock in [Clock::Realtime, Clock::Monotonic] {
            let t0 = Instant::now();
            let at = after(clock, Duration::from_millis(30));
            assert_eq!(m2.mutex().lock(&T, LockWait::Until(clock, &at)), Err(Errno::ETIMEDOUT));
            assert!(t0.elapsed() >= Duration::from_millis(25));
        }
        let bad = Timespec { sec: 0, nsec: 2_000_000_000 };
        assert_eq!(m2.mutex().lock(&T, LockWait::Until(Clock::Realtime, &bad)), Err(Errno::EINVAL), "an invalid time when it would block");
    })
    .join()
    .unwrap();
    m.mutex().unlock(&T).unwrap();
    let bad = Timespec { sec: 0, nsec: -1 };
    assert_eq!(m.mutex().lock(&T, LockWait::Until(Clock::Realtime, &bad)), Ok(()), "not checked when the mutex is free");
}

#[test]
fn a_signal_wakes_only_a_waiter_that_blocked_before_it() {
    // Clauses: s22_5_c_1_1, s22_5_c_1.
    // W1 waits; the main thread signals while holding the mutex; W2 starts waiting after the
    // signal, before W1 can run. W1 must wake; W2 must not (it times out).
    let m = Obj::new(10);
    let c = Obj::new(12);
    m.mutex().init(ps::MUTEX_NORMAL, 0);
    Cond::new(&c.w).init(0);
    let woke1 = Arc::new(AtomicU32::new(0));
    let ready = Arc::new(AtomicU32::new(0));
    let (m1, c1, w1, r1) = (Arc::clone(&m), Arc::clone(&c), Arc::clone(&woke1), Arc::clone(&ready));
    let h1 = std::thread::spawn(move || {
        m1.mutex().lock(&T, LockWait::Block).unwrap();
        r1.store(1, SeqCst);
        Cond::new(&c1.w).wait(&T, &m1.mutex(), None).unwrap();
        w1.store(1, SeqCst);
        m1.mutex().unlock(&T).unwrap();
    });
    while ready.load(SeqCst) == 0 {
        std::thread::yield_now();
    }
    // W1 holds the mutex until it waits; taking it here means W1 is waiting.
    m.mutex().lock(&T, LockWait::Block).unwrap();
    Cond::new(&c.w).signal(&T);
    let (m2, c2) = (Arc::clone(&m), Arc::clone(&c));
    let entered = Arc::new(AtomicU32::new(0));
    let e2 = Arc::clone(&entered);
    let h2 = std::thread::spawn(move || {
        m2.mutex().lock(&T, LockWait::Block).unwrap();
        e2.store(1, SeqCst);
        let at = after(Clock::Realtime, Duration::from_millis(200));
        let r = Cond::new(&c2.w).wait(&T, &m2.mutex(), Some((Clock::Realtime, &at)));
        m2.mutex().unlock(&T).unwrap();
        r
    });
    // Let W2 queue on the mutex before releasing it: it waits after the signal.
    std::thread::sleep(Duration::from_millis(20));
    m.mutex().unlock(&T).unwrap();
    h1.join().unwrap();
    assert_eq!(woke1.load(SeqCst), 1);
    assert_eq!(h2.join().unwrap(), Err(Errno::ETIMEDOUT), "the later waiter did not take the earlier signal");
    assert_eq!(entered.load(SeqCst), 1);
}

#[test]
fn broadcast_wakes_all_and_producer_consumer_loses_nothing() {
    // Clauses: s22_5_c_1.
    let m = Obj::new(10);
    let c = Obj::new(12);
    m.mutex().init(ps::MUTEX_NORMAL, 0);
    Cond::new(&c.w).init(ps::COND_MONOTONIC);
    let items = Arc::new(AtomicU32::new(0));
    let taken = Arc::new(AtomicU32::new(0));
    let consumers: Vec<_> = (0..6)
        .map(|_| {
            let (m, c, items, taken) = (Arc::clone(&m), Arc::clone(&c), Arc::clone(&items), Arc::clone(&taken));
            std::thread::spawn(move || loop {
                m.mutex().lock(&T, LockWait::Block).unwrap();
                while items.load(SeqCst) == 0 {
                    if taken.load(SeqCst) >= 6000 {
                        m.mutex().unlock(&T).unwrap();
                        return;
                    }
                    Cond::new(&c.w).wait(&T, &m.mutex(), None).unwrap();
                }
                items.fetch_sub(1, SeqCst);
                taken.fetch_add(1, SeqCst);
                m.mutex().unlock(&T).unwrap();
            })
        })
        .collect();
    for _ in 0..6000 {
        m.mutex().lock(&T, LockWait::Block).unwrap();
        items.fetch_add(1, SeqCst);
        Cond::new(&c.w).signal(&T);
        m.mutex().unlock(&T).unwrap();
    }
    while taken.load(SeqCst) < 6000 {
        std::thread::yield_now();
    }
    m.mutex().lock(&T, LockWait::Block).unwrap();
    Cond::new(&c.w).broadcast(&T);
    m.mutex().unlock(&T).unwrap();
    for h in consumers {
        h.join().unwrap();
    }
    assert_eq!(taken.load(SeqCst), 6000);
    let at = after(Clock::Monotonic, Duration::from_millis(10));
    m.mutex().lock(&T, LockWait::Block).unwrap();
    assert_eq!(Cond::new(&c.w).wait(&T, &m.mutex(), Some((Cond::new(&c.w).clock(), &at))), Err(Errno::ETIMEDOUT));
    m.mutex().unlock(&T).unwrap();
}

#[test]
fn rwlocks_share_readers_exclude_writers_and_detect_self_deadlock() {
    // Clauses: s22_5_c_2 (D-22.4).
    for kind in [ps::RW_PREFER_READER, ps::RW_PREFER_WRITER_NONRECURSIVE] {
        let l = Obj::new(14);
        RwLock::new(&l.w).init(kind, false);
        let readers = Arc::new(AtomicU32::new(0));
        let writers = Arc::new(AtomicU32::new(0));
        let hs: Vec<_> = (0..8)
            .map(|i| {
                let (l, readers, writers) = (Arc::clone(&l), Arc::clone(&readers), Arc::clone(&writers));
                std::thread::spawn(move || {
                    for _ in 0..2000 {
                        if i % 4 == 0 {
                            RwLock::new(&l.w).write(&T, LockWait::Block).unwrap();
                            assert_eq!(writers.fetch_add(1, SeqCst), 0);
                            assert_eq!(readers.load(SeqCst), 0);
                            writers.fetch_sub(1, SeqCst);
                        } else {
                            RwLock::new(&l.w).read(&T, LockWait::Block).unwrap();
                            readers.fetch_add(1, SeqCst);
                            assert_eq!(writers.load(SeqCst), 0);
                            readers.fetch_sub(1, SeqCst);
                        }
                        RwLock::new(&l.w).unlock(&T).unwrap();
                    }
                })
            })
            .collect();
        for h in hs {
            h.join().unwrap();
        }
        let r = RwLock::new(&l.w);
        r.write(&T, LockWait::Block).unwrap();
        assert_eq!(r.write(&T, LockWait::Block), Err(Errno::EDEADLK));
        assert_eq!(r.read(&T, LockWait::Block), Err(Errno::EDEADLK));
        // The try forms report a lock the caller holds for writing as busy (issue #15).
        assert_eq!(r.read(&T, LockWait::Try), Err(Errno::EBUSY));
        assert_eq!(r.write(&T, LockWait::Try), Err(Errno::EBUSY));
        assert_eq!(r.destroy(), Err(Errno::EBUSY));
        r.unlock(&T).unwrap();
        assert_eq!(r.unlock(&T), Err(Errno::EPERM));
        r.read(&T, LockWait::Block).unwrap();
        r.read(&T, LockWait::Block).unwrap(); // recursive read
        let l2 = Arc::clone(&l);
        std::thread::spawn(move || assert_eq!(RwLock::new(&l2.w).write(&T, LockWait::Try), Err(Errno::EBUSY))).join().unwrap();
        r.unlock(&T).unwrap();
        r.unlock(&T).unwrap();
    }
}

#[test]
fn barriers_choose_one_serial_thread_per_round() {
    // Clauses: s22_5_c_4.
    let b = Obj::new(8);
    assert_eq!(Barrier::new(&b.w).init(0, false), Err(Errno::EINVAL));
    Barrier::new(&b.w).init(5, false).unwrap();
    let serial = Arc::new(AtomicU32::new(0));
    let hs: Vec<_> = (0..5)
        .map(|_| {
            let (b, serial) = (Arc::clone(&b), Arc::clone(&serial));
            std::thread::spawn(move || {
                for _ in 0..500 {
                    if Barrier::new(&b.w).wait(&T) == ps::BARRIER_SERIAL {
                        serial.fetch_add(1, SeqCst);
                    }
                }
            })
        })
        .collect();
    for h in hs {
        h.join().unwrap();
    }
    assert_eq!(serial.load(SeqCst), 500, "exactly one per round");
}

#[test]
fn once_runs_once_and_a_reset_lets_it_run_again() {
    // Clauses: s22_5_c_5.
    let word = Arc::new(AtomicU32::new(0));
    let runs = Arc::new(AtomicU32::new(0));
    let hs: Vec<_> = (0..8)
        .map(|_| {
            let (word, runs) = (Arc::clone(&word), Arc::clone(&runs));
            std::thread::spawn(move || {
                ps::once(&CSys, &word, || {
                    std::thread::sleep(Duration::from_millis(10));
                    runs.fetch_add(1, SeqCst);
                });
                assert_eq!(runs.load(SeqCst), 1, "callers return after it completed");
            })
        })
        .collect();
    for h in hs {
        h.join().unwrap();
    }
    let w2 = AtomicU32::new(0);
    let mut n = 0;
    ps::once(&CSys, &w2, || ps::once_reset(&CSys, &AtomicU32::new(0)));
    ps::once(&CSys, &w2, || n += 1);
    assert_eq!(n, 0);
    ps::once_reset(&CSys, &w2);
    ps::once(&CSys, &w2, || n += 1);
    assert_eq!(n, 1, "after a reset (a cancelled routine) it runs again");
}

#[test]
fn semaphores_count_block_and_time_out() {
    // Clauses: s22_7_c_1, s22_7_c_2, s22_7_c_3, s22_7_f_0.
    let s = Obj::new(8);
    assert_eq!(Sem::new(&s.w).init(ps::SEM_VALUE_MAX + 1, false), Err(Errno::EINVAL));
    Sem::new(&s.w).init(0, false).unwrap();
    assert_eq!(Sem::new(&s.w).wait(&T, LockWait::Try), Err(Errno::EAGAIN));
    let at = after(Clock::Realtime, Duration::from_millis(20));
    assert_eq!(Sem::new(&s.w).wait(&T, LockWait::Until(Clock::Realtime, &at)), Err(Errno::ETIMEDOUT));
    let got = Arc::new(AtomicU32::new(0));
    let hs: Vec<_> = (0..4)
        .map(|_| {
            let (s, got) = (Arc::clone(&s), Arc::clone(&got));
            std::thread::spawn(move || {
                for _ in 0..1000 {
                    Sem::new(&s.w).wait(&T, LockWait::Block).unwrap();
                    got.fetch_add(1, SeqCst);
                }
            })
        })
        .collect();
    for _ in 0..4000 {
        Sem::new(&s.w).post(&CSys).unwrap();
    }
    for h in hs {
        h.join().unwrap();
    }
    assert_eq!(got.load(SeqCst), 4000);
    assert_eq!(Sem::new(&s.w).value(), 0);
    Sem::new(&s.w).init(ps::SEM_VALUE_MAX, false).unwrap();
    assert_eq!(Sem::new(&s.w).post(&CSys), Err(Errno::EOVERFLOW));
}

#[test]
fn a_priority_inheritance_mutex_never_takes_the_plain_fast_path() {
    // Clauses: s22_4_c_0, s22_4_c_2 (MUTEX-5).
    // MUTEX-5: the plain fast path is for plain kinds only. A PI mutex's lock word holds the
    // owner's TID (the kernel's protocol), not the plain 1.
    let p = Obj::new(10);
    p.mutex().init(ps::MUTEX_NORMAL | ps::KIND_PI, 0);
    p.mutex().lock(&T, LockWait::Block).unwrap();
    assert_eq!(p.w[0].load(SeqCst), T.tid(), "PI lock word");
    p.mutex().unlock(&T).unwrap();
    assert_eq!(p.w[0].load(SeqCst), 0);
}

/// A thread whose every futex wait the kernel refuses with one error (a rejected argument, a
/// seccomp filter), counting the waits. Its TID (1) is no other holder's.
struct Refused {
    err: Errno,
    waits: AtomicU32,
}

/// Refused waits a call may make before it is retrying forever (one is enough for every loop).
const RETRIES: u32 = 1000;

impl Ctx for Refused {
    type S = CSys;
    fn sys(&self) -> &CSys {
        &CSys
    }
    fn tid(&self) -> u32 {
        1
    }
    fn robust(&self) -> Option<&[AtomicUsize; 3]> {
        None
    }
    fn links(&self, _: usize) -> Option<&[AtomicUsize]> {
        None
    }
    fn wait(&self, _: &AtomicU32, _: u32, _: Scope, _: Option<(Clock, &Timespec)>, _: bool) -> KResult<()> {
        let n = self.waits.fetch_add(1, SeqCst) + 1;
        assert!(n <= RETRIES, "a refused wait ({:?}) was retried {n} times", self.err);
        Err(self.err)
    }
}

#[test]
fn a_wait_the_kernel_refuses_ends_the_call_with_its_error() {
    // no spec clause (TIMED-WAIT-PAST-DEADLINE): a wait that ends neither by a wake-up (EAGAIN,
    // EINTR) nor by ETIMEDOUT can never succeed when retried, so every blocking and timed lock,
    // condition-variable and semaphore wait returns the kernel's error instead of looping.
    let past = Timespec { sec: 1, nsec: 0 };
    for err in [Errno::EINVAL, Errno::ENOSYS, Errno::EFAULT] {
        for how in [LockWait::Block, LockWait::Until(Clock::Monotonic, &past), LockWait::Until(Clock::Realtime, &past)] {
            let r = Refused { err, waits: AtomicU32::new(0) };
            let deadline = match how {
                LockWait::Until(c, t) => Some((c, t)),
                _ => None,
            };
            // Mutexes another thread holds: plain, adaptive, priority-protected, robust.
            for kind in [ps::MUTEX_NORMAL, ps::MUTEX_ADAPTIVE, ps::KIND_PP, ps::KIND_ROBUST] {
                let m = Obj::new(10);
                m.mutex().init(kind, 0);
                m.mutex().lock(&T, LockWait::Block).unwrap();
                assert_eq!(m.mutex().lock(&r, how), Err(err), "mutex kind {kind}");
                m.mutex().unlock(&T).unwrap();
            }
            // A condition-variable wait, which returns with the mutex held.
            let (m, c) = (Obj::new(10), Obj::new(12));
            m.mutex().init(ps::MUTEX_NORMAL, 0);
            Cond::new(&c.w).init(0);
            m.mutex().lock(&r, LockWait::Block).unwrap();
            assert_eq!(Cond::new(&c.w).wait(&r, &m.mutex(), deadline), Err(err), "cond");
            m.mutex().unlock(&r).unwrap();
            // Read and write locks of a read-write lock held the other way.
            let l = Obj::new(14);
            RwLock::new(&l.w).init(ps::RW_PREFER_READER, false);
            RwLock::new(&l.w).write(&T, LockWait::Block).unwrap();
            assert_eq!(RwLock::new(&l.w).read(&r, how), Err(err), "rwlock read");
            RwLock::new(&l.w).unlock(&T).unwrap();
            RwLock::new(&l.w).read(&T, LockWait::Block).unwrap();
            assert_eq!(RwLock::new(&l.w).write(&r, how), Err(err), "rwlock write");
            RwLock::new(&l.w).unlock(&T).unwrap();
            // A semaphore at 0.
            let s = Obj::new(8);
            Sem::new(&s.w).init(0, false).unwrap();
            assert_eq!(Sem::new(&s.w).wait(&r, how), Err(err), "semaphore");
        }
    }
}

#[test]
fn a_deadline_with_a_negative_second_has_passed() {
    // Clauses: s22_4_c_5, s22_7_c_2 (TIMED-WAIT-PAST-DEADLINE): a negative tv_sec is before every
    // reading of either clock; the wait times out at once, where the kernel would refuse it.
    for clock in [Clock::Realtime, Clock::Monotonic] {
        for sec in [-1, i64::MIN] {
            let at = Timespec { sec, nsec: 0 };
            let m = Obj::new(10);
            m.mutex().init(ps::MUTEX_NORMAL, 0);
            m.mutex().lock(&T, LockWait::Block).unwrap();
            let m2 = Arc::clone(&m);
            let r = std::thread::spawn(move || m2.mutex().lock(&T, LockWait::Until(clock, &at))).join().unwrap();
            assert_eq!(r, Err(Errno::ETIMEDOUT), "contended mutex, {clock:?} {sec}");
            m.mutex().unlock(&T).unwrap();
            assert_eq!(m.mutex().lock(&T, LockWait::Until(clock, &at)), Ok(()), "a free mutex is taken");
            m.mutex().unlock(&T).unwrap();
            let s = Obj::new(8);
            Sem::new(&s.w).init(0, false).unwrap();
            assert_eq!(Sem::new(&s.w).wait(&T, LockWait::Until(clock, &at)), Err(Errno::ETIMEDOUT), "semaphore at 0, {clock:?} {sec}");
        }
    }
}

#[test]
fn an_invalid_deadline_is_an_error_only_when_the_mutex_would_block() {
    // Clauses: s22_4_c_5, s22_7_c_2 (TIMED-WAIT-EDGE-ORDER-2, D-22.26): a tv_nsec outside
    // 0..999999999 is not looked at when the mutex is free (every kind takes it, through its own
    // try path); a held one gives EINVAL whatever tv_sec, a past one included. A semaphore reports
    // EINVAL even when it could be taken, and keeps its value.
    let bad = [Timespec { sec: -1, nsec: 1_000_000_000 }, Timespec { sec: -1, nsec: -1 }, Timespec { sec: 5, nsec: 1_000_000_000 }];
    let kinds = [
        ps::MUTEX_NORMAL,
        ps::MUTEX_RECURSIVE,
        ps::MUTEX_ERRORCHECK,
        ps::MUTEX_ADAPTIVE,
        ps::MUTEX_NORMAL | ps::KIND_ROBUST,
        ps::MUTEX_NORMAL | ps::KIND_PP,
        ps::MUTEX_NORMAL | ps::KIND_PI,
        ps::MUTEX_NORMAL | ps::KIND_PI | ps::KIND_ROBUST,
    ];
    for clock in [Clock::Realtime, Clock::Monotonic] {
        for at in bad {
            for kind in kinds {
                let m = Obj::new(10);
                m.mutex().init(kind, 1);
                assert_eq!(m.mutex().lock(&T, LockWait::Until(clock, &at)), Ok(()), "free, kind {kind:#x}, {clock:?} {at:?}");
                m.mutex().unlock(&T).unwrap();
                // Held by this thread; a PI kind's other thread meets it in the kernel's try-lock.
                m.mutex().lock(&T, LockWait::Block).unwrap();
                let m2 = Arc::clone(&m);
                let r = std::thread::spawn(move || m2.mutex().lock(&T, LockWait::Until(clock, &at))).join().unwrap();
                assert_eq!(r, Err(Errno::EINVAL), "held, kind {kind:#x}, {clock:?} {at:?}");
                m.mutex().unlock(&T).unwrap();
            }
            let s = Obj::new(8);
            Sem::new(&s.w).init(1, false).unwrap();
            assert_eq!(Sem::new(&s.w).wait(&T, LockWait::Until(clock, &at)), Err(Errno::EINVAL), "semaphore at 1, {clock:?} {at:?}");
            assert_eq!(Sem::new(&s.w).wait(&T, LockWait::Try), Ok(()), "the value is kept");
        }
    }
}
