/* libthread_db.so.1 (spec §22.10; decision record SYS-THREAD-DB-2-1): the library a debugger loads
   into its own process to list a libniji program's threads, live or in a core file. It reads the
   target only through the debugger's ps_* callbacks, starting at the descriptor libniji's C
   library exports as `__niji_thread_db` (niji_rt::thread::Debug: 17 words, the list head's
   address, the live count's, the TLS image's, and the offsets of this build's layout). It
   imports nothing else: the debugger's process runs whatever C library the system has.

   Types follow the installed thread_db.h / proc_service.h interface (x86-64 layouts). */

#include <stddef.h>
#include <stdint.h>

typedef enum {
    TD_OK, TD_ERR, TD_NOTHR, TD_NOSV, TD_NOLWP, TD_BADPH, TD_BADTH, TD_BADSH, TD_BADTA, TD_BADKEY,
    TD_NOMSG, TD_NOFPREGS, TD_NOLIBTHREAD, TD_NOEVENT, TD_NOCAPAB, TD_DBERR, TD_NOAPLIC, TD_NOTSD,
    TD_MALLOC, TD_PARTIALREG, TD_NOXREGS, TD_TLSDEFER, TD_VERSION, TD_NOTLS
} td_err_e;
enum { TD_THR_ANY_STATE, TD_THR_UNKNOWN, TD_THR_STOPPED, TD_THR_RUN, TD_THR_ACTIVE, TD_THR_ZOMBIE };
enum { TD_THR_USER = 1 };

struct ps_prochandle;
typedef struct td_thragent td_thragent_t;
typedef struct td_thrhandle { td_thragent_t *th_ta_p; void *th_unique; } td_thrhandle_t;
typedef struct { uint32_t event_bits[2]; } td_thr_events_t;
typedef struct { unsigned long val[16]; } td_sigset_t;
typedef struct td_thrinfo {
    td_thragent_t *ti_ta_p;
    unsigned int ti_user_flags;
    unsigned long ti_tid;
    char *ti_tls;
    void *ti_startfunc;
    void *ti_stkbase;
    long ti_stksize;
    void *ti_ro_area;
    int ti_ro_size;
    int ti_state;
    unsigned char ti_db_suspended;
    int ti_type;
    intptr_t ti_pc;
    intptr_t ti_sp;
    short ti_flags;
    int ti_pri;
    int ti_lid;
    td_sigset_t ti_sigmask;
    unsigned char ti_traceme, ti_preemptflag, ti_pirecflag;
    td_sigset_t ti_pending;
    td_thr_events_t ti_events;
} td_thrinfo_t;
typedef struct {
    int nthreads, r_concurrency, nrunnable_num, nrunnable_den, a_concurrency_num, a_concurrency_den;
    int nlwps_num, nlwps_den, nidle_num, nidle_den;
} td_ta_stats_t;
typedef int td_thr_iter_f(const td_thrhandle_t *, void *);
_Static_assert(sizeof(td_thrinfo_t) == 384, "td_thrinfo_t is 384 bytes on x86-64");
_Static_assert(offsetof(td_thrinfo_t, ti_lid) == 104, "ti_lid offset");

/* The debugger's callbacks (PS_OK is 0). */
extern int ps_pdread(struct ps_prochandle *, void *, void *, size_t);
extern int ps_lgetregs(struct ps_prochandle *, int, uint64_t *);
extern int ps_lsetregs(struct ps_prochandle *, int, const uint64_t *);
extern int ps_lgetfpregs(struct ps_prochandle *, int, void *);
extern int ps_lsetfpregs(struct ps_prochandle *, int, const void *);
extern int ps_pglobal_lookup(struct ps_prochandle *, const char *, const char *, void **);
extern int ps_getpid(struct ps_prochandle *);

/* The descriptor's words (niji_rt::thread::Debug, version 1). */
enum {
    D_MAGIC, D_VERSION, D_WORDS, D_LIST, D_LIVE, D_TLS_IMAGE, D_TB_TID, D_TB_DTV, D_NEXT, D_TP, D_FLAGS,
    D_START, D_STACK_LO, D_STACK_SIZE, D_PRIO, D_EXITED, D_DTV_HEADER, D_COUNT
};
#define DEBUG_MAGIC 0x31626474696a696eULL /* "nijitdb1", little-endian */
#define DEBUG_VERSION 1
/* user_regs_struct (kernel UAPI asm/user.h): fs_base is word 21 of 27. */
#define REG_WORDS 27
#define REG_FS_BASE 21
/* Longest list walked: a list damaged in a core file must not hang the debugger. */
#define WALK_MAX (1u << 20)
#define AGENTS 64

struct td_thragent { struct ps_prochandle *ph; uint64_t d[D_COUNT]; };
static struct td_thragent agents[AGENTS];
static int agent_used[AGENTS];

static const char *const symbols[] = { "__niji_thread_db", NULL };

static int rd64(const td_thragent_t *ta, uint64_t addr, uint64_t *out) {
    return ps_pdread(ta->ph, (void *)(uintptr_t)addr, out, sizeof *out) == 0;
}
static int rd32(const td_thragent_t *ta, uint64_t addr, uint32_t *out) {
    return ps_pdread(ta->ph, (void *)(uintptr_t)addr, out, sizeof *out) == 0;
}

/* The listed descriptor whose thread pointer is tp: 1 found, 0 not listed, -1 unreadable. */
static int find(const td_thragent_t *ta, uint64_t tp, uint64_t *desc) {
    uint64_t d, t;
    unsigned n = 0;
    if (!rd64(ta, ta->d[D_LIST], &d)) return -1;
    for (; d != 0 && n < WALK_MAX; n++) {
        if (!rd64(ta, d + ta->d[D_TP], &t)) return -1;
        if (t == tp) { *desc = d; return 1; }
        if (!rd64(ta, d + ta->d[D_NEXT], &d)) return -1;
    }
    return 0;
}

/* The kernel thread ID and state of thread tp (descriptor desc, 0: not listed yet). */
static int state_of(const td_thragent_t *ta, uint64_t tp, uint64_t desc, uint32_t *tid) {
    uint32_t flags = 0;
    if (!rd32(ta, tp + ta->d[D_TB_TID], tid)) return TD_THR_UNKNOWN;
    if (desc != 0 && !rd32(ta, desc + ta->d[D_FLAGS], &flags)) return TD_THR_UNKNOWN;
    return (*tid == 0 || (flags & ta->d[D_EXITED]) != 0) ? TD_THR_ZOMBIE : TD_THR_ACTIVE;
}

td_err_e niji_ph_td_init(void) { return TD_OK; }
td_err_e niji_ph_td_log(void) { return TD_OK; }
const char **niji_ph_td_symbol_list(void) { return (const char **)symbols; }

td_err_e niji_ph_td_ta_new(struct ps_prochandle *ph, td_thragent_t **out) {
    void *at = NULL;
    uint64_t head[3];
    if (ph == NULL || out == NULL) return TD_BADPH;
    if (ps_pglobal_lookup(ph, "libc.so.6", symbols[0], &at) != 0 || at == NULL) return TD_NOLIBTHREAD;
    if (ps_pdread(ph, at, head, sizeof head) != 0) return TD_ERR;
    if (head[D_MAGIC] != DEBUG_MAGIC) return TD_NOLIBTHREAD;
    if (head[D_VERSION] != DEBUG_VERSION || head[D_WORDS] < D_COUNT) return TD_VERSION;
    for (int i = 0; i < AGENTS; i++) {
        if (__atomic_exchange_n(&agent_used[i], 1, __ATOMIC_ACQ_REL) != 0) continue;
        td_thragent_t *ta = &agents[i];
        ta->ph = ph;
        if (ps_pdread(ph, at, ta->d, sizeof ta->d) != 0) {
            __atomic_store_n(&agent_used[i], 0, __ATOMIC_RELEASE);
            return TD_ERR;
        }
        *out = ta;
        return TD_OK;
    }
    return TD_MALLOC;
}

td_err_e niji_ph_td_ta_delete(td_thragent_t *ta) {
    if (ta < agents || ta >= agents + AGENTS) return TD_BADTA;
    __atomic_store_n(&agent_used[ta - agents], 0, __ATOMIC_RELEASE);
    return TD_OK;
}

td_err_e niji_ph_td_ta_get_ph(const td_thragent_t *ta, struct ps_prochandle **ph) {
    if (ta == NULL) return TD_BADTA;
    *ph = ta->ph;
    return TD_OK;
}

td_err_e niji_ph_td_ta_get_nthreads(const td_thragent_t *ta, int *n) {
    uint64_t live;
    if (ta == NULL) return TD_BADTA;
    if (!rd64(ta, ta->d[D_LIVE], &live)) return TD_ERR;
    *n = (int)live;
    return TD_OK;
}

td_err_e niji_ph_td_ta_map_lwp2thr(const td_thragent_t *ta, int lwp, td_thrhandle_t *th) {
    uint64_t regs[REG_WORDS];
    if (ta == NULL) return TD_BADTA;
    if (ps_lgetregs(ta->ph, lwp, regs) != 0) return TD_ERR;
    /* Before the startup sets the thread pointer only the initial thread runs: handle 0. */
    th->th_ta_p = (td_thragent_t *)ta;
    th->th_unique = (void *)(uintptr_t)regs[REG_FS_BASE];
    return TD_OK;
}

td_err_e niji_ph_td_ta_map_id2thr(const td_thragent_t *ta, unsigned long id, td_thrhandle_t *th) {
    uint64_t desc;
    if (ta == NULL) return TD_BADTA;
    int r = find(ta, id, &desc);
    if (r < 0) return TD_ERR;
    if (r == 0) return TD_NOTHR;
    th->th_ta_p = (td_thragent_t *)ta;
    th->th_unique = (void *)(uintptr_t)id;
    return TD_OK;
}

td_err_e niji_ph_td_ta_thr_iter(const td_thragent_t *ta, td_thr_iter_f *cb, void *data, int state, int pri, void *mask, unsigned flags) {
    uint64_t d, tp;
    uint32_t tid;
    unsigned n = 0;
    (void)pri, (void)mask, (void)flags;
    if (ta == NULL) return TD_BADTA;
    if (!rd64(ta, ta->d[D_LIST], &d)) return TD_ERR;
    for (; d != 0 && n < WALK_MAX; n++) {
        if (!rd64(ta, d + ta->d[D_TP], &tp)) return TD_ERR;
        int st = state_of(ta, tp, d, &tid);
        if (state == TD_THR_ANY_STATE || state == st) {
            td_thrhandle_t th = { (td_thragent_t *)ta, (void *)(uintptr_t)tp };
            if (cb(&th, data) != 0) return TD_DBERR;
        }
        if (!rd64(ta, d + ta->d[D_NEXT], &d)) return TD_ERR;
    }
    return TD_OK;
}

td_err_e niji_ph_td_thr_validate(const td_thrhandle_t *th) {
    uint64_t desc;
    if (th == NULL || th->th_ta_p == NULL) return TD_BADTH;
    int r = find(th->th_ta_p, (uintptr_t)th->th_unique, &desc);
    return r < 0 ? TD_ERR : r == 0 ? TD_NOTHR : TD_OK;
}

td_err_e niji_ph_td_thr_get_info(const td_thrhandle_t *th, td_thrinfo_t *info) {
    uint64_t desc = 0, tp, v;
    uint32_t tid, prio;
    if (th == NULL || th->th_ta_p == NULL || info == NULL) return TD_BADTH;
    const td_thragent_t *ta = th->th_ta_p;
    tp = (uintptr_t)th->th_unique;
    unsigned char *p = (unsigned char *)info;
    for (size_t i = 0; i < sizeof *info; i++) p[i] = 0;
    info->ti_ta_p = (td_thragent_t *)ta;
    info->ti_type = TD_THR_USER;
    if (tp == 0) {
        /* The initial thread before its thread pointer is set. */
        info->ti_state = TD_THR_ACTIVE;
        info->ti_lid = ps_getpid(ta->ph);
        return TD_OK;
    }
    if (find(ta, tp, &desc) < 0) return TD_ERR;
    info->ti_tid = tp;
    info->ti_state = state_of(ta, tp, desc, &tid);
    info->ti_lid = (int)tid;
    if (desc != 0) {
        if (rd64(ta, desc + ta->d[D_START], &v)) info->ti_startfunc = (void *)(uintptr_t)v;
        if (rd64(ta, desc + ta->d[D_STACK_LO], &v) && v != 0) {
            uint64_t size = 0;
            if (rd64(ta, desc + ta->d[D_STACK_SIZE], &size)) {
                info->ti_stkbase = (void *)(uintptr_t)(v + size);
                info->ti_stksize = (long)size;
            }
        }
        if (rd32(ta, desc + ta->d[D_PRIO], &prio)) info->ti_pri = (int)prio;
    }
    return TD_OK;
}

/* The LWP of a handle, for the register calls. */
static td_err_e lwp_of(const td_thrhandle_t *th, int *lwp) {
    td_thrinfo_t info;
    td_err_e e = niji_ph_td_thr_get_info(th, &info);
    if (e != TD_OK) return e;
    if (info.ti_state == TD_THR_ZOMBIE || info.ti_lid <= 0) return TD_NOTHR;
    *lwp = info.ti_lid;
    return TD_OK;
}

td_err_e niji_ph_td_thr_getgregs(const td_thrhandle_t *th, uint64_t *regs) {
    int lwp;
    td_err_e e = lwp_of(th, &lwp);
    return e != TD_OK ? e : ps_lgetregs(th->th_ta_p->ph, lwp, regs) == 0 ? TD_OK : TD_ERR;
}
td_err_e niji_ph_td_thr_setgregs(const td_thrhandle_t *th, const uint64_t *regs) {
    int lwp;
    td_err_e e = lwp_of(th, &lwp);
    return e != TD_OK ? e : ps_lsetregs(th->th_ta_p->ph, lwp, regs) == 0 ? TD_OK : TD_ERR;
}
td_err_e niji_ph_td_thr_getfpregs(const td_thrhandle_t *th, void *regs) {
    int lwp;
    td_err_e e = lwp_of(th, &lwp);
    return e != TD_OK ? e : ps_lgetfpregs(th->th_ta_p->ph, lwp, regs) == 0 ? TD_OK : TD_ERR;
}
td_err_e niji_ph_td_thr_setfpregs(const td_thrhandle_t *th, const void *regs) {
    int lwp;
    td_err_e e = lwp_of(th, &lwp);
    return e != TD_OK ? e : ps_lsetfpregs(th->th_ta_p->ph, lwp, regs) == 0 ? TD_OK : TD_ERR;
}

/* Module modid's TLS block in thread th: through the thread's dtv when the loader built one,
   else (a static program: one module, its block ending at the thread pointer) from the TLS image. */
td_err_e niji_ph_td_thr_tlsbase(const td_thrhandle_t *th, unsigned long modid, void **base) {
    uint64_t tp, dtv, n, block, img[4];
    if (th == NULL || th->th_ta_p == NULL || base == NULL) return TD_BADTH;
    const td_thragent_t *ta = th->th_ta_p;
    tp = (uintptr_t)th->th_unique;
    if (tp == 0) return TD_TLSDEFER;
    if (modid == 0) return TD_NOTLS;
    if (!rd64(ta, tp + ta->d[D_TB_DTV], &dtv)) return TD_ERR;
    if (dtv != 0) {
        if (!rd64(ta, dtv + 8, &n)) return TD_ERR;
        if (modid > n) return TD_TLSDEFER;
        if (!rd64(ta, dtv + 8 * (ta->d[D_DTV_HEADER] + modid - 1), &block)) return TD_ERR;
        if (block == 0) return TD_TLSDEFER;
        *base = (void *)(uintptr_t)block;
        return TD_OK;
    }
    if (modid != 1) return TD_NOTLS;
    if (ps_pdread(ta->ph, (void *)(uintptr_t)ta->d[D_TLS_IMAGE], img, sizeof img) != 0) return TD_ERR;
    if (img[2] == 0) return TD_NOTLS;
    uint64_t align = img[3] > 1 ? img[3] : 1;
    uint64_t size = (img[2] + align - 1) & ~(align - 1);
    *base = (void *)(uintptr_t)(tp - size);
    return TD_OK;
}

/* A module's TLS by its link_map: libniji's loader publishes no link_map list a debugger can
   read yet, so there is no module ID to find (static programs use td_thr_tlsbase). */
td_err_e niji_ph_td_thr_tls_get_addr(const td_thrhandle_t *th, void *map, size_t offset, void **addr) {
    (void)th, (void)map, (void)offset, (void)addr;
    return TD_NOCAPAB;
}

td_err_e niji_ph_td_ta_get_stats(const td_thragent_t *ta, td_ta_stats_t *s) {
    int n;
    td_err_e e = niji_ph_td_ta_get_nthreads(ta, &n);
    if (e != TD_OK) return e;
    *s = (td_ta_stats_t){ .nthreads = n };
    return TD_OK;
}
td_err_e niji_ph_td_ta_enable_stats(const td_thragent_t *ta, int on) { (void)on; return ta == NULL ? TD_BADTA : TD_OK; }
td_err_e niji_ph_td_ta_reset_stats(const td_thragent_t *ta) { return ta == NULL ? TD_BADTA : TD_OK; }

/* Event reporting: the debugger learns of new and exiting threads from the kernel (ptrace
   clone and exit events); libniji has no event breakpoints to report. */
td_err_e niji_ph_td_ta_event_addr(const td_thragent_t *ta, int ev, void *n) { (void)ta, (void)ev, (void)n; return TD_NOEVENT; }
td_err_e niji_ph_td_ta_set_event(const td_thragent_t *ta, td_thr_events_t *e) { (void)ta, (void)e; return TD_NOCAPAB; }
td_err_e niji_ph_td_ta_clear_event(const td_thragent_t *ta, td_thr_events_t *e) { (void)ta, (void)e; return TD_NOCAPAB; }
td_err_e niji_ph_td_ta_event_getmsg(const td_thragent_t *ta, void *m) { (void)ta, (void)m; return TD_NOMSG; }
td_err_e niji_ph_td_thr_event_enable(const td_thrhandle_t *th, int on) { (void)th, (void)on; return TD_NOCAPAB; }
td_err_e niji_ph_td_thr_set_event(const td_thrhandle_t *th, td_thr_events_t *e) { (void)th, (void)e; return TD_NOCAPAB; }
td_err_e niji_ph_td_thr_clear_event(const td_thrhandle_t *th, td_thr_events_t *e) { (void)th, (void)e; return TD_NOCAPAB; }
td_err_e niji_ph_td_thr_event_getmsg(const td_thrhandle_t *th, void *m) { (void)th, (void)m; return TD_NOMSG; }

/* Not offered: thread-specific data, extra register sets, and acting on a thread. */
td_err_e niji_ph_td_ta_tsd_iter(const td_thragent_t *ta, void *cb, void *data) { (void)ta, (void)cb, (void)data; return TD_NOCAPAB; }
td_err_e niji_ph_td_thr_tsd(const td_thrhandle_t *th, unsigned key, void **v) { (void)th, (void)key, (void)v; return TD_NOCAPAB; }
td_err_e niji_ph_td_ta_setconcurrency(const td_thragent_t *ta, int n) { (void)ta, (void)n; return TD_NOCAPAB; }
td_err_e niji_ph_td_thr_getxregsize(const td_thrhandle_t *th, int *n) { (void)th, (void)n; return TD_NOXREGS; }
td_err_e niji_ph_td_thr_getxregs(const td_thrhandle_t *th, void *x) { (void)th, (void)x; return TD_NOXREGS; }
td_err_e niji_ph_td_thr_setxregs(const td_thrhandle_t *th, const void *x) { (void)th, (void)x; return TD_NOXREGS; }
td_err_e niji_ph_td_thr_dbsuspend(const td_thrhandle_t *th) { (void)th; return TD_NOCAPAB; }
td_err_e niji_ph_td_thr_dbresume(const td_thrhandle_t *th) { (void)th; return TD_NOCAPAB; }
td_err_e niji_ph_td_thr_setprio(const td_thrhandle_t *th, int p) { (void)th, (void)p; return TD_NOCAPAB; }
td_err_e niji_ph_td_thr_setsigpending(const td_thrhandle_t *th, unsigned char n, const void *s) { (void)th, (void)n, (void)s; return TD_NOCAPAB; }
td_err_e niji_ph_td_thr_sigsetmask(const td_thrhandle_t *th, const void *s) { (void)th, (void)s; return TD_NOCAPAB; }
