/* Spec §22.7 Contract (s22_7_c_5): a process may have many named semaphores open at once; no
 * count of its own limits them (HL-7: libniji once refused the 257th with EMFILE). 2000 names are
 * opened at once, each a semaphore of its own (its own address and value) that opening its name
 * again gives back; once every name is closed, all 2000 open again at once by name with the
 * values they were left with, close again, and are unlinked. Any failure unlinks every name
 * before it reports. */
#include <errno.h>
#include <fcntl.h>
#include <semaphore.h>
#include <stdio.h>
#include <unistd.h>

enum { N = 2000 };
static sem_t *sems[N];
static char names[N][48];

static int fail(int rc, int i)
{
    printf("s22_sem_named_many: step %d failed at %d (errno %d)\n", rc, i, errno);
    for (int k = 0; k < N; k++)
        sem_unlink(names[k]);
    return rc;
}

int main(void)
{
    for (int i = 0; i < N; i++)
        snprintf(names[i], sizeof names[i], "/niji-conf-many-%ld-%d", (long)getpid(), i);

    for (int i = 0; i < N; i++) {
        sems[i] = sem_open(names[i], O_CREAT | O_EXCL, 0600, (unsigned)(i % 7));
        if (sems[i] == SEM_FAILED)
            return fail(1, i);
    }
    /* Each one usable, with its own value. */
    for (int i = 0; i < N; i++) {
        int v = -1;
        if (sem_post(sems[i]) != 0 || sem_getvalue(sems[i], &v) != 0 || v != i % 7 + 1)
            return fail(2, i);
    }
    /* Opening a name again gives the same semaphore. */
    for (int i = 0; i < N; i++)
        if (sem_open(names[i], 0) != sems[i])
            return fail(3, i);
    /* No two names share an address. */
    for (int i = 1; i < N; i++)
        for (int j = 0; j < i; j++)
            if (sems[i] == sems[j])
                return fail(4, i);
    /* Each was opened twice: two closes each. */
    for (int i = 0; i < N; i++)
        if (sem_close(sems[i]) != 0 || sem_close(sems[i]) != 0)
            return fail(5, i);

    /* All of them again at once, by name, with the values they were left with. */
    for (int i = 0; i < N; i++) {
        sems[i] = sem_open(names[i], 0);
        if (sems[i] == SEM_FAILED)
            return fail(6, i);
    }
    for (int i = 0; i < N; i++) {
        int v = -1;
        if (sem_getvalue(sems[i], &v) != 0 || v != i % 7 + 1 || sem_trywait(sems[i]) != 0)
            return fail(7, i);
    }
    for (int i = 0; i < N; i++)
        if (sem_close(sems[i]) != 0)
            return fail(8, i);

    for (int i = 0; i < N; i++)
        if (sem_unlink(names[i]) != 0)
            return fail(9, i);
    for (int i = 0; i < N; i++)
        if (sem_open(names[i], 0) != SEM_FAILED || errno != ENOENT)
            return fail(10, i);
    return 0;
}
