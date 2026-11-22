/* SHSTK-SCOPE bench and probe (scratch, not in the tree). Usage: shb <mode> [n] [arg]
 * modes: empty, status, thread N, longjmp N, swap N, signal N, mapss N, ssinfo, dlopen PATH,
 * vfork N, deep N.  Prints nothing but the requested lines, so counts include little I/O. */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <pthread.h>
#include <setjmp.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <ucontext.h>
#include <unistd.h>

static unsigned long rdssp(void)
{
	unsigned long v = 0;
	__asm__ volatile("rdsspq %0" : "+r"(v));
	return v;
}

static void status(const char *tag)
{
	char buf[8192];
	FILE *f = fopen("/proc/self/status", "r");
	if (!f)
		return;
	while (fgets(buf, sizeof buf, f))
		if (!strncmp(buf, "x86_Thread", 10))
			printf("%s %s", tag, buf);
	fclose(f);
	printf("%s ssp=%s\n", tag, rdssp() ? "nonzero" : "0");
}

static void ssinfo(const char *tag)
{
	/* the mappings whose VmFlags hold "ss" (shadow stack): size and Rss */
	char buf[512], range[128] = "";
	unsigned long size = 0, rss = 0;
	FILE *f = fopen("/proc/self/smaps", "r");
	if (!f)
		return;
	while (fgets(buf, sizeof buf, f)) {
		unsigned long a, b;
		if (sscanf(buf, "%lx-%lx", &a, &b) == 2 && strchr(buf, ' ') && buf[strlen(buf) - 1] == '\n' && strstr(buf, " ") && !strncmp(buf + strcspn(buf, " "), " ", 1)) {
			snprintf(range, sizeof range, "%lx-%lx", a, b);
		}
		if (sscanf(buf, "Size: %lu kB", &size) == 1)
			continue;
		if (sscanf(buf, "Rss: %lu kB", &rss) == 1)
			continue;
		if (!strncmp(buf, "VmFlags:", 8) && strstr(buf, " ss"))
			printf("%s shstk %s size=%lu kB rss=%lu kB\n", tag, range, size, rss);
	}
	fclose(f);
}

static volatile int sink;
static void *thr(void *a) { sink++; return a; }
static void *thr_info(void *a) { ssinfo("thread"); status("thread"); return a; }

static void cleanup(void *a) { (void)a; sink++; }
static __attribute__((noinline)) void exit_deep(int d)
{
	if (d == 0)
		pthread_exit((void *)7);
	pthread_cleanup_push(cleanup, 0);
	exit_deep(d - 1);
	pthread_cleanup_pop(0);
}
static void *thr_exit(void *a) { exit_deep(5); return a; }
static void *thr_cancel(void *a)
{
	pthread_cleanup_push(cleanup, 0);
	for (;;)
		pause();
	pthread_cleanup_pop(0);
	return a;
}

static jmp_buf jb;
static __attribute__((noinline)) void deep_jump(int d)
{
	if (d == 0)
		longjmp(jb, 1);
	deep_jump(d - 1);
	sink++;
}

static ucontext_t uc_main, uc_co;
static long swaps;
static void co(void)
{
	for (;;) {
		swaps++;
		swapcontext(&uc_co, &uc_main);
	}
}

static void on_sig(int s) { (void)s; sink++; }

int main(int argc, char **argv)
{
	const char *m = argc > 1 ? argv[1] : "empty";
	long n = argc > 2 ? atol(argv[2]) : 0;
	if (!strcmp(m, "empty"))
		return 0;
	if (!strcmp(m, "status")) {
		status("main");
		return 0;
	}
	if (!strcmp(m, "ssinfo")) {
		status("main");
		ssinfo("main");
		pthread_t t;
		pthread_create(&t, 0, thr_info, 0);
		pthread_join(t, 0);
		return 0;
	}
	if (!strcmp(m, "thread")) {
		for (long i = 0; i < n; i++) {
			pthread_t t;
			if (pthread_create(&t, 0, thr, 0))
				return 2;
			pthread_join(t, 0);
		}
		return 0;
	}
	if (!strcmp(m, "longjmp")) {
		for (long i = 0; i < n; i++)
			if (!setjmp(jb))
				longjmp(jb, 1);
		return 0;
	}
	if (!strcmp(m, "deep")) {	/* longjmp out of 10 frames */
		for (long i = 0; i < n; i++)
			if (!setjmp(jb))
				deep_jump(10);
		return 0;
	}
	if (!strcmp(m, "swap")) {
		static char stk[65536];
		getcontext(&uc_co);
		uc_co.uc_stack.ss_sp = stk;
		uc_co.uc_stack.ss_size = sizeof stk;
		uc_co.uc_link = 0;
		makecontext(&uc_co, co, 0);
		if (rdssp() && getenv("NJSHSTK")) {	/* libniji prototype only */
			/* PROTOTYPE: the bench gives the context its shadow stack (453 =
			 * map_shadow_stack, flag 1 = SHADOW_STACK_SET_TOKEN: a restore token at the top) */
			unsigned long sz = 1 << 16;
			long a = syscall(453, 0, sz, 1);
			if (a < 0 && a > -4096)
				return 3;
			uc_co.__ssp[0] = a + sz - 8;
			uc_co.__ssp[1] = 0;
		}
		for (long i = 0; i < n; i++)
			swapcontext(&uc_main, &uc_co);
		return swaps == n ? 0 : 4;
	}
	if (!strcmp(m, "mapss")) {
		for (long i = 0; i < n; i++) {
			long a = syscall(453, 0, 1 << 16, 1);
			if (a < 0 && a > -4096)
				return 3;
			munmap((void *)a, 1 << 16);
		}
		return 0;
	}
	if (!strcmp(m, "signal")) {
		struct sigaction sa;
		memset(&sa, 0, sizeof sa);
		sa.sa_handler = on_sig;
		sigaction(SIGUSR1, &sa, 0);
		for (long i = 0; i < n; i++)
			raise(SIGUSR1);
		return sink == n ? 0 : 5;
	}
	if (!strcmp(m, "vfork")) {
		for (long i = 0; i < n; i++) {
			pid_t p = vfork();
			if (p == 0)
				_exit(0);
			int st;
			waitpid(p, &st, 0);
		}
		return 0;
	}
	if (!strcmp(m, "pexit") || !strcmp(m, "cancel")) {
		int c = !strcmp(m, "cancel");
		for (long i = 0; i < n; i++) {
			pthread_t t;
			void *r;
			if (pthread_create(&t, 0, c ? thr_cancel : thr_exit, 0))
				return 2;
			if (c)
				pthread_cancel(t);
			pthread_join(t, &r);
			if (r != (c ? PTHREAD_CANCELED : (void *)7))
				return 6;
		}
		return 0;
	}
	if (!strcmp(m, "dlopen")) {
		status("before");
		void *h = dlopen(argc > 2 ? argv[2] : "libunm.so", RTLD_NOW);
		printf("dlopen %s: %s\n", argc > 2 ? argv[2] : "?", h ? "ok" : dlerror());
		status("after");
		return 0;
	}
	return 9;
}
