/* Asynchronous I/O through <aio.h> (§11.11, s11_11_c_0): a lio_listio batch with LIO_WAIT that writes and
 * skips (LIO_NOP), a second batch that reads the data back, aio_error and aio_return on each request,
 * aio_suspend on finished requests, aio_cancel on a finished request (AIO_ALLDONE), and a single aio_write,
 * aio_fsync and aio_read polled to completion. Each operation names its request by the header's constants,
 * so a constant with the wrong value turns a write into a read (or a no-op) and the data check fails.
 * Built on libniji's <aio.h> (the header's own test); by hand it also passes built on the reference's. */
/* NIJI-HEADERS: yes */
#define _GNU_SOURCE
#include <aio.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <sys/mman.h>
#include <unistd.h>

static int fail(int code, const char *what)
{
    fprintf(stderr, "FAIL %d: %s (errno %d)\n", code, what, errno);
    return code;
}

static void setup(struct aiocb *cb, int fd, int op, void *buf, size_t n, off_t off)
{
    memset(cb, 0, sizeof *cb);
    cb->aio_fildes = fd;
    cb->aio_lio_opcode = op;
    cb->aio_buf = buf;
    cb->aio_nbytes = n;
    cb->aio_offset = off;
    cb->aio_sigevent.sigev_notify = SIGEV_NONE;
}

/* Wait for one request with aio_suspend, then check its status and its count. */
static int finish(struct aiocb *cb, ssize_t want)
{
    const struct aiocb *list[1] = { cb };
    struct timespec ts = { 30, 0 };
    while (aio_error(cb) == EINPROGRESS)
        if (aio_suspend(list, 1, &ts) != 0 && errno != EINTR)
            return 0;
    return aio_error(cb) == 0 && aio_return(cb) == want;
}

int main(void)
{
    /* A memory file: aio_fsync on a disk file would wait for the file system's journal. */
    int fd = memfd_create("s11_aio_listio", 0);
    if (fd < 0)
        return fail(1, "memfd_create");

    if (LIO_READ == LIO_WRITE || LIO_READ == LIO_NOP || LIO_WRITE == LIO_NOP)
        return fail(2, "LIO_READ, LIO_WRITE and LIO_NOP are not distinct");
    if (AIO_CANCELED == AIO_NOTCANCELED || AIO_CANCELED == AIO_ALLDONE || AIO_NOTCANCELED == AIO_ALLDONE)
        return fail(3, "AIO_CANCELED, AIO_NOTCANCELED and AIO_ALLDONE are not distinct");
    if (LIO_WAIT == LIO_NOWAIT)
        return fail(4, "LIO_WAIT equals LIO_NOWAIT");

    /* Batch 1: two writes at different offsets and a no-op, waited for by lio_listio itself. */
    char a[] = "hello, ", b[] = "world";
    struct aiocb w1, w2, nop;
    setup(&w1, fd, LIO_WRITE, a, 7, 0);
    setup(&w2, fd, LIO_WRITE, b, 5, 7);
    setup(&nop, fd, LIO_NOP, NULL, 0, 0);
    struct aiocb *batch1[3] = { &w1, &nop, &w2 };
    if (lio_listio(LIO_WAIT, batch1, 3, NULL) != 0)
        return fail(5, "lio_listio LIO_WAIT write batch");
    if (aio_error(&w1) != 0 || aio_return(&w1) != 7)
        return fail(6, "first write's status");
    if (aio_error(&w2) != 0 || aio_return(&w2) != 5)
        return fail(7, "second write's status");

    /* The file holds what the writes put there (read synchronously). */
    char sync_buf[16] = { 0 };
    if (pread(fd, sync_buf, sizeof sync_buf, 0) != 12 || memcmp(sync_buf, "hello, world", 12) != 0)
        return fail(8, "the write batch did not write the data");

    /* Batch 2: read it back in two pieces. */
    char r1[8] = { 0 }, r2[8] = { 0 };
    struct aiocb rd1, rd2;
    setup(&rd1, fd, LIO_READ, r1, 5, 0);
    setup(&rd2, fd, LIO_READ, r2, 5, 7);
    struct aiocb *batch2[2] = { &rd1, &rd2 };
    if (lio_listio(LIO_WAIT, batch2, 2, NULL) != 0)
        return fail(9, "lio_listio LIO_WAIT read batch");

    /* Before their status is retrieved: aio_suspend on finished requests returns at once, and aio_cancel
     * of a finished one is AIO_ALLDONE. */
    const struct aiocb *done[2] = { &rd1, &rd2 };
    struct timespec zero = { 0, 0 };
    if (aio_suspend(done, 2, &zero) != 0)
        return fail(12, "aio_suspend on finished requests");
    if (aio_cancel(fd, &rd1) != AIO_ALLDONE)
        return fail(13, "aio_cancel of a finished request is not AIO_ALLDONE");

    if (aio_error(&rd1) != 0 || aio_return(&rd1) != 5 || memcmp(r1, "hello", 5) != 0)
        return fail(10, "first read");
    if (aio_error(&rd2) != 0 || aio_return(&rd2) != 5 || memcmp(r2, "world", 5) != 0)
        return fail(11, "second read");

    /* LIO_NOWAIT: the batch is queued and each request finishes on its own. */
    char c[] = "AIO";
    struct aiocb w3;
    setup(&w3, fd, LIO_WRITE, c, 3, 12);
    struct aiocb *batch3[1] = { &w3 };
    if (lio_listio(LIO_NOWAIT, batch3, 1, NULL) != 0)
        return fail(14, "lio_listio LIO_NOWAIT");
    if (!finish(&w3, 3))
        return fail(15, "LIO_NOWAIT write");

    /* Single requests: aio_write, aio_fsync (O_SYNC), aio_read, each polled to completion. */
    char d[] = "!";
    struct aiocb w4, fs, rd3;
    setup(&w4, fd, 0, d, 1, 15);
    if (aio_write(&w4) != 0 || !finish(&w4, 1))
        return fail(16, "aio_write");
    setup(&fs, fd, 0, NULL, 0, 0);
    if (aio_fsync(O_SYNC, &fs) != 0 || !finish(&fs, 0))
        return fail(17, "aio_fsync");
    char all[17] = { 0 };
    setup(&rd3, fd, 0, all, 16, 0);
    if (aio_read(&rd3) != 0 || !finish(&rd3, 16) || memcmp(all, "hello, worldAIO!", 16) != 0)
        return fail(18, "aio_read of the whole file");

    close(fd);
    printf("s11_aio_listio: ok\n");
    return 0;
}
