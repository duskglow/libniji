/* Spec §23.6 (s23_6_c_0, s23_6_c_2) and spec/security/threat-model.md (the TZ row): in secure-execution
 * mode (here a setgid program) a TZ value names a zone file only inside /usr/share/zoneinfo. A zone
 * name, a POSIX rule and an absolute path inside the directory (after empty and `.` components are
 * dropped) select the zone; an absolute path outside it, or a value with a `..` component, reads no
 * file and is taken as a rule: offset 0, abbreviated by the value's leading valid name (empty if none).
 * The outside file is a copy of Asia/Tokyo the program makes, so only the path refuses it. Every line
 * is printed, so the program is also the black-box probe of the reference (spec/behaviour/
 * dynlink-secure.md). Needs Asia/Tokyo in the system zone directory (skipped, with the reason, if not). */
/* RUN-SECURE: yes */
#define _GNU_SOURCE
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/auxv.h>
#include <time.h>
#include <unistd.h>

static const char tokyo[] = "/usr/share/zoneinfo/Asia/Tokyo";
static int bad;

/* Select `tz` and check localtime_r of 1970-06-29 (Tokyo: +9 h, JST, no DST then). */
static void expect(int line, const char *tz, long off, const char *zone)
{
    time_t t = 15206400;
    struct tm tm;
    setenv("TZ", tz, 1);
    tzset();
    if (!localtime_r(&t, &tm)) {
        printf("TZ=%s localtime_r failed\n", tz);
        bad = line;
        return;
    }
    const char *z = tm.tm_zone ? tm.tm_zone : "(null)";
    printf("TZ=%s gmtoff %ld zone '%s'\n", tz, (long)tm.tm_gmtoff, z);
    if (tm.tm_gmtoff != off || strcmp(z, zone) != 0) {
        printf("  line %d: want gmtoff %ld zone '%s'\n", line, off, zone);
        bad = line;
    }
}

#define EXPECT(tz, off, zone) expect(__LINE__, (tz), (off), (zone))

/* Copy `from` to `to`; 0 on success. */
static int copy(const char *from, const char *to)
{
    char buf[4096];
    int in = open(from, O_RDONLY | O_CLOEXEC), out = open(to, O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC, 0644);
    ssize_t n = 0;
    if (in < 0 || out < 0)
        return -1;
    while ((n = read(in, buf, sizeof buf)) > 0)
        if (write(out, buf, (size_t)n) != n)
            n = -1;
    close(in);
    return close(out) != 0 || n < 0 ? -1 : 0;
}

int main(void)
{
    char dir[] = "/tmp/niji-tzsec-XXXXXX", outside[64], colon[80], up[96];
    if (getauxval(AT_SECURE) != 1) {
        printf("not in secure mode\n");
        return 1;
    }
    if (access(tokyo, R_OK) != 0) {
        printf("conformance: SKIP s23_tz_secure (%s is not readable here)\n", tokyo);
        return 0;
    }
    if (!mkdtemp(dir))
        return 2;
    snprintf(outside, sizeof outside, "%s/myzone", dir);
    snprintf(colon, sizeof colon, ":%s", outside);
    snprintf(up, sizeof up, "../../../..%s", outside);
    if (copy(tokyo, outside) != 0)
        return 3;

    /* A zone name, a rule, a path inside the directory. */
    EXPECT("Asia/Tokyo", 32400, "JST");
    EXPECT(":Asia/Tokyo", 32400, "JST");
    EXPECT("JST-9", 32400, "JST");
    EXPECT(tokyo, 32400, "JST");
    EXPECT(":/usr/share/zoneinfo/Asia/Tokyo", 32400, "JST");
    EXPECT("/usr/share/zoneinfo//Asia/./Tokyo", 32400, "JST");
    /* Outside the directory, or through `..`: no file read. */
    EXPECT(outside, 0, "");
    EXPECT(colon, 0, "");
    EXPECT(up, 0, "");
    EXPECT("Asia/../Asia/Tokyo", 0, "Asia");
    EXPECT("/usr/share/zoneinfo/../zoneinfo/Asia/Tokyo", 0, "");
    /* A rule followed by a `..` component: still no file read; the value is a rule. */
    EXPECT("JST-9/../x", 32400, "JST");

    unlink(outside);
    rmdir(dir);
    return bad;
}
