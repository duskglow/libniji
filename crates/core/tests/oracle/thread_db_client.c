/* A debugger-shaped client of libthread_db (crates/core/tests/thread_db_products.rs): it includes
   <thread_db.h> and <proc_service.h> as gdb's thread support does, defines every <proc_service.h>
   callback a debugger supplies with the header's own prototypes (a definition that disagrees with
   the declaration does not compile), and calls td_init, td_ta_new and (behind a run-time branch it
   never takes) td_thr_get_info. Its "target" has no symbols: ps_pglobal_lookup answers PS_NOSYM,
   so td_ta_new must refuse it. It prints the interface's sizes, member offsets and event-set macro
   results, which must be the same built on the reference's headers and library and on libniji's. */
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <sys/types.h>
#include <thread_db.h>
#include <proc_service.h>

struct ps_prochandle {
    int lookups;
};

ps_err_e ps_pglobal_lookup(struct ps_prochandle *ph, const char *obj, const char *name, psaddr_t *addr) {
    (void) obj;
    (void) name;
    (void) addr;
    ph->lookups++;
    return PS_NOSYM;
}

ps_err_e ps_pdread(struct ps_prochandle *ph, psaddr_t addr, void *buf, size_t size) {
    (void) ph;
    (void) addr;
    memset(buf, 0, size);
    return PS_ERR;
}

ps_err_e ps_pdwrite(struct ps_prochandle *ph, psaddr_t addr, const void *buf, size_t size) {
    (void) ph;
    (void) addr;
    (void) buf;
    (void) size;
    return PS_ERR;
}

ps_err_e ps_lgetregs(struct ps_prochandle *ph, lwpid_t lwp, prgregset_t regs) {
    (void) ph;
    (void) lwp;
    (void) regs;
    return PS_ERR;
}

ps_err_e ps_lsetregs(struct ps_prochandle *ph, lwpid_t lwp, const prgregset_t regs) {
    (void) ph;
    (void) lwp;
    (void) regs;
    return PS_ERR;
}

ps_err_e ps_lgetfpregs(struct ps_prochandle *ph, lwpid_t lwp, prfpregset_t *regs) {
    (void) ph;
    (void) lwp;
    (void) regs;
    return PS_ERR;
}

ps_err_e ps_lsetfpregs(struct ps_prochandle *ph, lwpid_t lwp, const prfpregset_t *regs) {
    (void) ph;
    (void) lwp;
    (void) regs;
    return PS_ERR;
}

pid_t ps_getpid(struct ps_prochandle *ph) {
    (void) ph;
    return 1;
}

ps_err_e ps_get_thread_area(struct ps_prochandle *ph, lwpid_t lwp, int idx, psaddr_t *base) {
    (void) ph;
    (void) lwp;
    (void) idx;
    (void) base;
    return PS_ERR;
}

int main(int argc, char **argv) {
    (void) argv;
    struct ps_prochandle ph = { 0 };
    td_thragent_t *ta = NULL;
    printf("td_init %s\n", td_init() == TD_OK ? "ok" : "failed");
    td_err_e e = td_ta_new(&ph, &ta);
    printf("td_ta_new %s, %s\n", e == TD_OK ? "accepted a target without symbols" : "refused", ph.lookups > 0 ? "looked up" : "no lookup");
    if (argc > 99) {
        /* Never run: a debugger's next calls, which must link. */
        td_thrhandle_t th;
        td_thrinfo_t info;
        if (td_ta_map_lwp2thr(ta, 1, &th) == TD_OK && td_thr_get_info(&th, &info) == TD_OK)
            printf("%ld\n", (long) info.ti_lid);
        td_ta_delete(ta);
    }
    td_thr_events_t ev;
    td_event_emptyset(&ev);
    printf("emptyset empty %d\n", td_eventisempty(&ev) ? 1 : 0);
    td_event_addset(&ev, TD_CREATE);
    td_event_addset(&ev, TD_DEATH);
    printf("create %d death %d reap %d empty %d\n", td_eventismember(&ev, TD_CREATE) != 0, td_eventismember(&ev, TD_DEATH) != 0,
           td_eventismember(&ev, TD_REAP) != 0, td_eventisempty(&ev) ? 1 : 0);
    td_event_delset(&ev, TD_CREATE);
    printf("after delset create %d death %d\n", td_eventismember(&ev, TD_CREATE) != 0, td_eventismember(&ev, TD_DEATH) != 0);
    td_event_fillset(&ev);
    printf("fillset create %d timeout %d\n", td_eventismember(&ev, TD_CREATE) != 0, td_eventismember(&ev, TD_TIMEOUT) != 0);
    printf("sizeof td_thrhandle_t %zu td_thrinfo_t %zu td_event_msg_t %zu td_notify_t %zu td_thr_events_t %zu\n",
           sizeof(td_thrhandle_t), sizeof(td_thrinfo_t), sizeof(td_event_msg_t), sizeof(td_notify_t), sizeof(td_thr_events_t));
    printf("td_thrhandle_t th_ta_p %zu th_unique %zu\n", offsetof(td_thrhandle_t, th_ta_p), offsetof(td_thrhandle_t, th_unique));
    printf("td_thrinfo_t ti_tid %zu ti_state %zu ti_lid %zu ti_events %zu\n", offsetof(td_thrinfo_t, ti_tid),
           offsetof(td_thrinfo_t, ti_state), offsetof(td_thrinfo_t, ti_lid), offsetof(td_thrinfo_t, ti_events));
    printf("TD_OK %d TD_NOTHR %d TD_VERSION %d TD_THR_ACTIVE %d TD_THR_SYSTEM %d TD_EVENTS_ENABLE %d PS_NOSYM %d\n",
           (int) TD_OK, (int) TD_NOTHR, (int) TD_VERSION, (int) TD_THR_ACTIVE, (int) TD_THR_SYSTEM, (int) TD_EVENTS_ENABLE,
           (int) PS_NOSYM);
    return 0;
}
