//! Reloading time-zone and locale data on request (D-X.13): `niji_tz_invalidate` and
//! `niji_locale_invalidate`. Zones and locale files stay pinned until then; after an
//! invalidation the next use compares each file with what was read, reuses what did not change
//! (no growth, the same pointers) and reads again what did. Each scenario changes process-wide
//! state (`TZ`, `LOCPATH`, the global locale), so it runs in a re-executed child of this test
//! binary; the fixtures (copies of installed zone files, locales compiled with `localedef`) are
//! in a scratch directory under `CARGO_TARGET_TMPDIR`.
#![no_builtins] // calls to libc names (labs, strlen, ...) reach libniji's export, not LLVM's folding (OPAQUE-AUDIT)
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::as_conversions, missing_docs)]
#[path = "../../core/tests/support/skip_note.rs"]
mod skip_note;

use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use niji_abi::layout::struct_tm;
use niji_boundary::native::locale::niji_locale_invalidate;
use niji_boundary::native::tz::niji_tz_invalidate;
use niji_boundary::posix::s01_startup as env;
use niji_boundary::posix::s04_strings as st;
use niji_boundary::posix::s07_locale as loc;
use niji_boundary::posix::s23_time as tm;

#[path = "../../core/tests/support/scratch.rs"]
mod scratch;

const PROBE: &str = "NIJI_RELOAD_PROBE";
const LC_ALL: i32 = 6;
const DAY_1: i32 = 0x20007;
/// 2026-07-01 12:00:00 UTC: summer in Berlin (+2 h), Tokyo +9 h all year.
const T: i64 = 1_782_907_200;

/// Run the test `name` again in a child with the probe set and `dir` passed; the child's
/// output must end with `ok`.
fn in_child(name: &str, dir: &Path) {
    let o = std::process::Command::new(std::env::current_exe().unwrap())
        .args([name, "--exact", "--include-ignored", "--test-threads=1", "--nocapture"])
        .env(PROBE, dir)
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&o.stdout).contains("running 1 test"), "the child ran no test (libtest filtered it out): {:?}", o.status);
    let out = String::from_utf8_lossy(&o.stdout);
    for l in out.lines().filter(|l| l.contains("loaded ")) {
        println!("{l}");
    }
    assert!(o.status.success() && out.contains("child: ok"), "{name} in its child: {out}\n{}", String::from_utf8_lossy(&o.stderr));
}

fn probe() -> Option<PathBuf> {
    std::env::var_os(PROBE).map(PathBuf::from)
}

fn c(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn set(name: &str, value: &str) {
    assert_eq!(env::setenv(c(name).as_ptr(), c(value).as_ptr(), 1), 0);
}

/// Replace the file at `to` with a copy of `from` (a new inode, as a package update does).
fn replace(from: &Path, to: &Path) {
    let tmp = to.with_extension("new");
    std::fs::copy(from, &tmp).unwrap();
    std::fs::rename(&tmp, to).unwrap();
}

fn zero_tm() -> struct_tm {
    struct_tm { tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 0, tm_mon: 0, tm_year: 0, tm_wday: 0, tm_yday: 0, tm_isdst: 0, tm_gmtoff: 0, tm_zone: 0 }
}

/// The NUL-terminated text at `addr` (read through the library's own `strlen` and `memcpy`).
fn text(addr: usize) -> Vec<u8> {
    let p = core::ptr::with_exposed_provenance::<core::ffi::c_char>(addr);
    let n = st::strlen(p);
    let mut v = vec![0u8; n];
    st::memcpy(v.as_mut_ptr().cast(), p.cast(), n);
    v
}

/// `localtime_r(T)`: the offset and the address of the abbreviation.
fn local() -> (i64, usize) {
    let mut out = zero_tm();
    assert!(!tm::localtime_r(Some(&T), Some(&mut out)).is_null());
    (out.tm_gmtoff, out.tm_zone)
}

/// `localtime(T)` (an implicit tzset): the offset.
fn implicit() -> i64 {
    let p = tm::localtime(Some(&T));
    assert!(!p.is_null());
    let mut out = zero_tm();
    st::memcpy(core::ptr::from_mut(&mut out).cast(), p.cast_const().cast(), size_of::<struct_tm>());
    out.tm_gmtoff
}

fn zone_fixture(d: &Path) -> PathBuf {
    let zone = d.join("zone");
    std::fs::copy("/usr/share/zoneinfo/Europe/Berlin", &zone).unwrap();
    zone
}

#[test]
fn tz_invalidate_reloads_a_changed_zone_file() {
    // Clauses: s23_6_c_0, s23_6_c_2, s23_6_g_0 (D-X.13, D-23.6).
    if let Some(d) = probe() {
        let zone = d.join("zone");
        set("TZ", &format!(":{}", zone.display()));
        tm::tzset();
        let (berlin, abbr) = local();
        assert_eq!(berlin, 7200);
        // Nothing changed: the same zone, the same abbreviation pointer (no new copy).
        let checked = std::time::Instant::now();
        assert_eq!(niji_tz_invalidate(), 0);
        assert_eq!(local(), (7200, abbr), "an unchanged file is reused");
        // Replaced: an implicit selection re-checks the file of the current zone at most once a second (D-23.6), so it
        // is pinned until an invalidation only while less than that has passed since the last check. How long the
        // steps take is the machine's business (a loaded host took 1.4 s here): the pin is asserted only when the
        // elapsed time proves no check was due (half the interval, for the coarse clock the check reads), and a
        // later implicit selection may already see the new file.
        replace(Path::new("/usr/share/zoneinfo/Asia/Tokyo"), &zone);
        let seen = implicit();
        if checked.elapsed() < std::time::Duration::from_millis(500) {
            assert_eq!(seen, 7200, "still the pinned zone (under the recheck interval since the last check)");
        } else {
            assert!(seen == 7200 || seen == 32400, "the pinned or the replaced zone, not a third: {seen}");
        }
        assert_eq!(niji_tz_invalidate(), 0);
        assert_eq!(local().0, 32400, "read again after the invalidation");
        assert_eq!(implicit(), 32400);
        // The old abbreviation is still readable (never freed).
        assert_eq!(text(abbr), b"CEST");
        // A removed file: the zone is read again from the rule fallback (UTC named by the value).
        std::fs::remove_file(&zone).unwrap();
        assert_eq!(niji_tz_invalidate(), 0);
        assert_eq!(local().0, 0, "a missing file is missing");
        println!("child: ok");
        return;
    }
    let d = scratch::Scratch::dir("reload-tz");
    zone_fixture(&d);
    in_child("tz_invalidate_reloads_a_changed_zone_file", &d);
}

/// Compile `name` (e.g. `en_US`) with `localedef` into `dir/<name>.UTF-8`; false without
/// `localedef`.
fn compile(dir: &Path, name: &str) -> bool {
    compile_locale_atomic(&dir.join(format!("{name}.UTF-8")), name, "UTF-8", "LC_TIME")
}

fn day1() -> Vec<u8> {
    text(loc::nl_langinfo(DAY_1).expose_provenance())
}

#[test]
fn locale_invalidate_reloads_a_changed_locale_file() {
    // Clauses: s7_1_c_0, s7_2_c_0, s7_2_c_1 (D-X.13, D-7.7).
    if let Some(d) = probe() {
        set("LOCPATH", &d.join("locales").display().to_string());
        let xx = d.join("locales/xx_XX.UTF-8");
        assert!(!loc::setlocale(LC_ALL, c("xx_XX.UTF-8").as_ptr()).is_null(), "the fixture locale loads");
        assert_eq!(day1(), b"Sunday");
        let before = loc::nl_langinfo(DAY_1).expose_provenance();
        assert_eq!(niji_locale_invalidate(), 0);
        assert_eq!((day1(), loc::nl_langinfo(DAY_1).expose_provenance()), (b"Sunday".to_vec(), before), "unchanged files are reused");
        // A replaced category file: pinned until an invalidation.
        replace(&d.join("locales/de_DE.UTF-8/LC_TIME"), &xx.join("LC_TIME"));
        assert!(!loc::setlocale(LC_ALL, c("xx_XX.UTF-8").as_ptr()).is_null());
        assert_eq!(day1(), b"Sunday", "still the pinned file");
        assert_eq!(niji_locale_invalidate(), 0);
        assert_eq!(day1(), b"Sonntag", "the global locale is loaded again");
        assert_eq!(text(before), b"Sunday", "the old item stays mapped");
        // A locale that disappeared: ENOENT, the global locale unchanged.
        std::fs::remove_dir_all(&xx).unwrap();
        assert_eq!(niji_locale_invalidate(), -2);
        assert_eq!(day1(), b"Sonntag");
        println!("child: ok");
        return;
    }
    let d = scratch::Scratch::dir("reload-locale");
    let locales = d.join("locales");
    std::fs::create_dir_all(&locales).unwrap();
    if !(compile(&locales, "en_US") && compile(&locales, "de_DE")) {
        skip_note::skipped("skipped: no localedef");
        return;
    }
    std::fs::rename(locales.join("en_US.UTF-8"), locales.join("xx_XX.UTF-8")).unwrap();
    in_child("locale_invalidate_reloads_a_changed_locale_file", &d);
}

#[test]
fn reloads_race_with_readers() {
    // Clauses: s23_6_g_0, s7_2_c_0, s4_4_c_0 (D-X.13): conversions, nl_langinfo and strcoll in
    // other threads while the files are replaced and invalidated repeatedly: every answer is
    // one of the two valid ones, and nothing faults.
    if let Some(d) = probe() {
        let zone = d.join("zone");
        set("TZ", &format!(":{}", zone.display()));
        set("LOCPATH", &d.join("locales").display().to_string());
        tm::tzset();
        assert!(!loc::setlocale(LC_ALL, c("xx_XX.UTF-8").as_ptr()).is_null());
        let stop = AtomicBool::new(false);
        let seen = AtomicUsize::new(0);
        std::thread::scope(|s| {
            for _ in 0..4 {
                s.spawn(|| {
                    let (a, b) = (c("apple"), c("Banana"));
                    while !stop.load(Ordering::Relaxed) {
                        let off = local().0;
                        assert!(off == 7200 || off == 32400, "offset {off}");
                        let day = day1();
                        assert!(day == b"Sunday" || day == b"Sonntag", "{day:?}");
                        assert!(st::strcoll(a.as_ptr(), b.as_ptr()) < 0);
                        seen.fetch_add(1, Ordering::Relaxed);
                    }
                });
            }
            let swaps = [("/usr/share/zoneinfo/Asia/Tokyo", "de_DE.UTF-8"), ("/usr/share/zoneinfo/Europe/Berlin", "en_US.UTF-8")];
            for k in 0..60 {
                let (z, l) = swaps[k % 2];
                replace(Path::new(z), &zone);
                replace(&d.join("locales").join(l).join("LC_TIME"), &d.join("locales/xx_XX.UTF-8/LC_TIME"));
                assert_eq!(niji_tz_invalidate(), 0);
                assert_eq!(niji_locale_invalidate(), 0);
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            stop.store(true, Ordering::Relaxed);
        });
        assert!(seen.load(Ordering::Relaxed) > 0);
        println!("child: ok");
        return;
    }
    let d = scratch::Scratch::dir("reload-race");
    zone_fixture(&d);
    let locales = d.join("locales");
    std::fs::create_dir_all(&locales).unwrap();
    if !(compile(&locales, "en_US") && compile(&locales, "de_DE")) {
        skip_note::skipped("skipped: no localedef");
        return;
    }
    let copied = std::process::Command::new("cp").arg("-r").arg(locales.join("en_US.UTF-8")).arg(locales.join("xx_XX.UTF-8")).status().unwrap();
    assert!(copied.success());
    in_child("reloads_race_with_readers", &d);
}

#[test]
fn a_fork_child_reloads_on_its_own() {
    // Clauses: s23_6_c_2, s23_6_g_0 (D-X.13): the child inherits the pinned zone and the
    // generation; its invalidation does not reach the parent, which keeps its zone until its own.
    if let Some(d) = probe() {
        let zone = d.join("zone");
        set("TZ", &format!(":{}", zone.display()));
        tm::tzset();
        assert_eq!(local().0, 7200);
        replace(Path::new("/usr/share/zoneinfo/Asia/Tokyo"), &zone);
        let child = |_: &niji_boundary::sys::CSys| -> i32 {
            let pinned = local().0 == 7200;
            let reloaded = niji_tz_invalidate() == 0 && local().0 == 32400;
            i32::from(!(pinned && reloaded))
        };
        assert_eq!(niji_boundary::selftest::in_child(child).unwrap(), niji_boundary::selftest::ChildEnd::Exited(0));
        assert_eq!(local().0, 7200, "the parent keeps its zone");
        assert_eq!(niji_tz_invalidate(), 0);
        assert_eq!(local().0, 32400);
        println!("child: ok");
        return;
    }
    let d = scratch::Scratch::dir("reload-fork");
    zone_fixture(&d);
    in_child("a_fork_child_reloads_on_its_own", &d);
}

#[test]
fn an_implicit_selection_sees_a_replaced_zone_file_after_the_interval() {
    // Clauses: s23_6_g_0, s23_6_c_2 (D-23.6, D-7.14's TSC gate): the once-a-second check of the current zone's
    // file is never skipped for good, however often the zone is used in between (the gate is primed first).
    if let Some(d) = probe() {
        let zone = d.join("zone");
        set("TZ", &format!(":{}", zone.display()));
        tm::tzset();
        for _ in 0..1000 {
            assert_eq!(implicit(), 7200);
        }
        replace(Path::new("/usr/share/zoneinfo/Asia/Tokyo"), &zone);
        // Past the one-second interval (the coarse clock's tick included); no invalidation.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        assert_eq!(implicit(), 32400, "the replaced file is seen by an implicit tzset once the interval has passed");
        println!("child: ok");
        return;
    }
    let d = scratch::Scratch::dir("reload-interval");
    zone_fixture(&d);
    in_child("an_implicit_selection_sees_a_replaced_zone_file_after_the_interval", &d);
}

#[test]
fn a_changed_tz_is_seen_by_the_next_implicit_selection() {
    // Clauses: s23_6_g_0, s23_6_c_0 (D-23.6): a changed `TZ` is compared on every implicit tzset, ahead of the
    // TSC gate, so it is seen at the next call however recently the zone file was checked.
    if let Some(d) = probe() {
        let zone = d.join("zone");
        let berlin = format!(":{}", zone.display());
        set("TZ", &berlin);
        for _ in 0..1000 {
            assert_eq!(implicit(), 7200);
        }
        set("TZ", ":/usr/share/zoneinfo/Asia/Tokyo");
        assert_eq!(implicit(), 32400, "the changed TZ, at once");
        set("TZ", &berlin);
        assert_eq!(implicit(), 7200, "and back, at once");
        set("TZ", "EST5EDT,M3.2.0,M11.1.0");
        assert_eq!(implicit(), -4 * 3600, "a rule value");
        println!("child: ok");
        return;
    }
    let d = scratch::Scratch::dir("reload-tz-change");
    zone_fixture(&d);
    in_child("a_changed_tz_is_seen_by_the_next_implicit_selection", &d);
}

const LC_ALL_MASK: i32 = 0x1FBF;
const CODESET: i32 = 14;
/// `_NL_LOCALE_NAME(LC_CTYPE)`.
const NL_LOCALE_NAME_CTYPE: i32 = 0xFFFF;
const RLIMIT_AS: i32 = 9;
const ENOMEM: i32 = 12;

/// The directory the many-locale tests link each locale to: the installed `C.utf8` (directory
/// form), else `en_US.UTF-8` compiled into `scratch`. `None`: neither is available.
fn link_target(scratch: &Path) -> Option<PathBuf> {
    let installed = Path::new("/usr/lib/locale/C.utf8");
    if installed.join("LC_CTYPE").exists() && installed.join("LC_TIME").exists() {
        return Some(installed.to_path_buf());
    }
    let compiled = scratch.join("compiled");
    std::fs::create_dir_all(&compiled).unwrap();
    compile(&compiled, "en_US").then(|| compiled.join("en_US.UTF-8"))
}

/// `n` locales `l<k>.UTF-8` under `scratch/many`, each a link to the same locale directory (each
/// file a distinct path), as the audit's probe made them. False (a skip, printed): no locale to
/// link to.
fn many_locales(scratch: &Path, n: usize) -> bool {
    let Some(target) = link_target(scratch) else {
        skip_note::skipped("skipped: no installed C.utf8 and no localedef");
        return false;
    };
    let many = scratch.join("many");
    std::fs::create_dir_all(&many).unwrap();
    for k in 0..n {
        std::os::unix::fs::symlink(&target, many.join(format!("l{k}.UTF-8"))).unwrap();
    }
    true
}

/// The lines of `/proc/self/maps` that map a file under `dir`.
fn mapped_under(dir: &Path) -> usize {
    let dir = dir.display().to_string();
    std::fs::read_to_string("/proc/self/maps").unwrap().lines().filter(|l| l.contains(dir.as_str())).count()
}

fn codeset(l: niji_abi::ctypes::locale_t) -> Vec<u8> {
    text(loc::nl_langinfo_l(CODESET, l).expose_provenance())
}

#[test]
fn locales_and_files_past_the_static_table_load() {
    // Clauses: s7_1_c_0, s7_2_c_0 (D-7.1, D-7.7): no limit on the locales, names and files a
    // process loads but memory. 1100 directory-form locales under a private LOCPATH: more
    // interned locales and names than the static table's 1024 entries, and their files (each a
    // distinct path) more than ten times as many mappings.
    const N: usize = 1100;
    if let Some(d) = probe() {
        set("LOCPATH", &d.join("many").display().to_string());
        let names: Vec<CString> = (0..N).map(|k| c(&format!("l{k}.UTF-8"))).collect();
        let before = niji_legacy::global_locale::mappings();
        let mut handles = Vec::with_capacity(N);
        for (k, n) in names.iter().enumerate() {
            let h = loc::newlocale(LC_ALL_MASK, n.as_ptr(), core::ptr::null_mut());
            assert!(!h.is_null(), "l{k} loads (errno {})", niji_boundary::errno::get());
            handles.push(h);
        }
        let made = niji_legacy::global_locale::mappings() - before;
        assert!(made > 2 * N, "each locale maps its own files: {made} mappings for {N} locales");
        // Each handle names its own interned locale (entries past the static table included).
        for (k, &h) in handles.iter().enumerate() {
            assert_eq!((text(loc::nl_langinfo_l(NL_LOCALE_NAME_CTYPE, h).expose_provenance()), codeset(h)), (names[k].as_bytes().to_vec(), b"UTF-8".to_vec()), "l{k}");
        }
        for n in [&names[0], &names[N - 1]] {
            let got = loc::setlocale(LC_ALL, n.as_ptr());
            assert!(!got.is_null(), "setlocale selects {n:?}");
            assert_eq!(text(got.expose_provenance()), n.as_bytes());
        }
        assert!(!loc::newlocale(LC_ALL_MASK, names[0].as_ptr(), core::ptr::null_mut()).is_null());
        assert_eq!(niji_legacy::global_locale::mappings() - before, made, "a locale loaded again maps nothing");
        assert!(!loc::setlocale(LC_ALL, c("C").as_ptr()).is_null());
        for h in handles {
            loc::freelocale(h);
        }
        println!("loaded {N}, mappings {made}");
        println!("child: ok");
        return;
    }
    let d = scratch::Scratch::dir("reload-many");
    if many_locales(&d, N) {
        in_child("locales_and_files_past_the_static_table_load", &d);
    }
}

/// The calling process's address-space size, in bytes (`VmSize`).
fn vm_size() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let kb = status.lines().find_map(|l| l.strip_prefix("VmSize:")).unwrap().trim().trim_end_matches("kB").trim();
    kb.parse::<u64>().unwrap() * 1024
}

/// Limit the address space to what is in use now plus `slack` bytes.
fn limit_address_space(slack: u64, max: u64) {
    let rl = niji_kernel::uapi::Rlimit { cur: vm_size() + slack, max };
    assert_eq!(niji_boundary::posix::s24_system::setrlimit(RLIMIT_AS, Some(&rl)), 0);
}

#[test]
fn a_load_that_runs_out_of_memory_fails_with_enomem() {
    // Clauses: s7_1_c_0, s7_2_c_0 (D-7.7): with an address-space limit too small for the
    // growth, `newlocale` and `setlocale` fail cleanly with ENOMEM (not as a missing locale), a
    // failed load repeated maps nothing more, the locales loaded before keep working, and the
    // same load succeeds once memory is available again.
    const N: usize = 1100;
    if let Some(d) = probe() {
        set("LOCPATH", &d.join("many").display().to_string());
        // Everything the limited phase needs is allocated before it.
        let names: Vec<CString> = (0..N).map(|k| c(&format!("l{k}.UTF-8"))).collect();
        let mut handles = Vec::with_capacity(N);
        let mut old = niji_kernel::uapi::Rlimit { cur: 0, max: 0 };
        assert_eq!(niji_boundary::posix::s24_system::getrlimit(RLIMIT_AS, Some(&mut old)), 0);
        limit_address_space(16 << 20, old.max);
        let mut failed = None;
        for (k, n) in names.iter().enumerate() {
            let h = loc::newlocale(LC_ALL_MASK, n.as_ptr(), core::ptr::null_mut());
            if h.is_null() {
                failed = Some((k, niji_boundary::errno::get()));
                break;
            }
            handles.push(h);
        }
        let made = niji_legacy::global_locale::mappings();
        let mut again = [0i32; 20];
        for e in &mut again {
            let h = loc::newlocale(LC_ALL_MASK, names[handles.len()].as_ptr(), core::ptr::null_mut());
            *e = if h.is_null() { niji_boundary::errno::get() } else { -1 };
        }
        let made_again = niji_legacy::global_locale::mappings();
        assert_eq!(niji_boundary::posix::s24_system::setrlimit(RLIMIT_AS, Some(&old)), 0);
        let (k, errno) = failed.expect("the address-space limit stops a load");
        assert_eq!(errno, ENOMEM, "newlocale of l{k} under the limit: ENOMEM, not a missing locale");
        assert_eq!(again, [ENOMEM; 20], "the load keeps failing the same way");
        assert_eq!(made_again, made, "a failed load repeated maps nothing more");
        for (j, &h) in handles.iter().enumerate() {
            assert_eq!(codeset(h), b"UTF-8", "l{j}, loaded before the failure, still works");
        }
        let h = loc::newlocale(LC_ALL_MASK, names[k].as_ptr(), core::ptr::null_mut());
        assert!(!h.is_null(), "l{k} loads once memory is available");
        // setlocale reports the same cause.
        let rest = &names[k + 1..];
        limit_address_space(4 << 20, old.max);
        let mut selected = 0;
        let mut set_errno = 0;
        for n in rest {
            if loc::setlocale(LC_ALL, n.as_ptr()).is_null() {
                set_errno = niji_boundary::errno::get();
                break;
            }
            selected += 1;
        }
        assert_eq!(niji_boundary::posix::s24_system::setrlimit(RLIMIT_AS, Some(&old)), 0);
        assert!(selected < rest.len(), "the address-space limit stops a setlocale");
        assert_eq!(set_errno, ENOMEM, "setlocale under the limit: ENOMEM");
        println!("loaded {k} before ENOMEM; setlocale selected {selected} more");
        println!("child: ok");
        return;
    }
    let d = scratch::Scratch::dir("reload-nomem");
    if many_locales(&d, N) {
        in_child("a_load_that_runs_out_of_memory_fails_with_enomem", &d);
    }
}

#[test]
fn repeated_updates_that_change_nothing_grow_nothing() {
    // Clauses: s7_1_c_0, s7_2_c_0 (D-X.13, D-7.7): 5000 cycles of replacing a category file of
    // the global locale with the same bytes (a new inode, as a reinstall makes) and invalidating:
    // every reload succeeds, the same mapping keeps serving and the process's mappings stay as
    // they were. A replacement with other bytes is still read.
    const CYCLES: usize = 5000;
    if let Some(d) = probe() {
        let locales = d.join("locales");
        set("LOCPATH", &locales.display().to_string());
        assert!(!loc::setlocale(LC_ALL, c("xx_XX.UTF-8").as_ptr()).is_null(), "the fixture locale loads");
        let file = locales.join("xx_XX.UTF-8/LC_TIME");
        let same = d.join("LC_TIME.same");
        let (maps, made, item) = (mapped_under(&locales), niji_legacy::global_locale::mappings(), loc::nl_langinfo(DAY_1).expose_provenance());
        for k in 0..CYCLES {
            replace(&same, &file);
            assert_eq!(niji_locale_invalidate(), 0, "reload {k}");
        }
        assert_eq!(niji_legacy::global_locale::mappings(), made, "no mapping is kept beside an identical one");
        assert_eq!(mapped_under(&locales), maps, "the locale files' address space is flat");
        assert_eq!((loc::nl_langinfo(DAY_1).expose_provenance(), day1()), (item, b"Sunday".to_vec()), "the same mapping serves");
        replace(&locales.join("de_DE.UTF-8/LC_TIME"), &file);
        assert_eq!(niji_locale_invalidate(), 0);
        assert_eq!(day1(), b"Sonntag", "other bytes are read");
        assert_eq!(niji_legacy::global_locale::mappings(), made + 1);
        println!("child: ok");
        return;
    }
    let d = scratch::Scratch::dir("reload-repeat");
    let locales = d.join("locales");
    std::fs::create_dir_all(&locales).unwrap();
    if !(compile(&locales, "en_US") && compile(&locales, "de_DE")) {
        skip_note::skipped("skipped: no localedef");
        return;
    }
    std::fs::rename(locales.join("en_US.UTF-8"), locales.join("xx_XX.UTF-8")).unwrap();
    std::fs::copy(locales.join("xx_XX.UTF-8/LC_TIME"), d.join("LC_TIME.same")).unwrap();
    in_child("repeated_updates_that_change_nothing_grow_nothing", &d);
}

/// Compile locale `lang`.`charset` into `dest` with the host's `localedef`, atomically: it is built
/// in a private sibling directory and renamed into place, so that concurrent tests and test
/// binaries never see (or write into) a half-built locale. True when `dest/<probe>` exists after.
fn compile_locale_atomic(dest: &std::path::Path, lang: &str, charset: &str, probe: &str) -> bool {
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    if dest.join(probe).exists() {
        return true;
    }
    let Some(parent) = dest.parent() else { return false };
    let _ = std::fs::create_dir_all(parent);
    let tmp = parent.join(format!(".build-{}-{}-{}", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::Relaxed), dest.file_name().unwrap().to_string_lossy()));
    let _ = std::fs::remove_dir_all(&tmp);
    // Absent (cannot be started) is a skip for the caller; present but failing is a failure (TEST-SILENT-SKIPS).
    let made = match std::process::Command::new("localedef").args(["-i", lang, "-f", charset]).arg(&tmp).output() {
        Err(_) => false,
        Ok(o) => {
            assert!(o.status.code().is_some_and(|c| c <= 1) && tmp.join(probe).exists(), "localedef is present but failed for {lang}.{charset} ({}): {}", o.status, String::from_utf8_lossy(&o.stderr));
            true
        }
    };
    if made && tmp.join(probe).exists() {
        // Losing the race (the destination appeared meanwhile) is fine: theirs is complete too.
        let _ = std::fs::rename(&tmp, dest);
    }
    let _ = std::fs::remove_dir_all(&tmp);
    dest.join(probe).exists()
}
