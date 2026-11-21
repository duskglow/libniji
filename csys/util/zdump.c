/* zdump: print the time, or the transitions, of time zones (spec §31.10, plan/utilities.md, story TZ-TOOLS).
 *
 * Written from zdump(8) and black-box runs of the reference command; no implementation of the tz tools was
 * consulted. A front end over the C library: each zone is selected with TZ and read back through
 * localtime_r and gmtime_r, so the program shows what any program linked with libniji sees. Built by
 * `cargo xtask so` into target/niji/zdump, shipped in libc-bin as /usr/bin/zdump.
 *
 * Transitions are found as zdump(8) documents: localtime is sampled every twelve hours over the range, and a
 * change of UT offset, DST flag or abbreviation between two samples is narrowed to the second it happens.
 * -v and -V print, for each transition, the second before it and the transition itself:
 *   ZONE  <UT date> UT = <local date> <abbr> isdst=<0|1> gmtoff=<seconds>
 * -v also prints the lowest and highest times and the times one day inside them (`= NULL` where the
 * library cannot represent the date). -i prints a compact table. The default range is the years -500 to
 * 2500 (-c) and can be given in seconds (-t). Zone names are padded to the longest one.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <inttypes.h>
#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static const char *progname = "zdump";

static void usage(FILE *f, int status) {
    fprintf(f,
            "%s: usage: %s OPTIONS TIMEZONE ...\n"
            "Options include:\n"
            "  -c [L,]U   Start at year L (default -500), end before year U (default 2500)\n"
            "  -t [L,]U   Start at time L, end before time U (in seconds since 1970)\n"
            "  -i         List transitions briefly (format is experimental)\n"
            "  -v         List transitions verbosely\n"
            "  -V         List transitions a bit less verbosely\n"
            "  --help     Output this help\n"
            "  --version  Output version info\n",
            progname, progname);
    exit(status);
}

/* What a reader of the zone sees at one instant. */
struct state {
    int ok;
    long gmtoff;
    int isdst;
    char abbr[64];
    struct tm tm;
};

static void sample(time_t t, struct state *s) {
    s->ok = localtime_r(&t, &s->tm) != NULL;
    if (!s->ok) return;
    s->gmtoff = s->tm.tm_gmtoff;
    s->isdst = s->tm.tm_isdst;
    snprintf(s->abbr, sizeof s->abbr, "%s", s->tm.tm_zone ? s->tm.tm_zone : "");
}

static int same(const struct state *a, const struct state *b) {
    if (a->ok != b->ok) return 0;
    if (!a->ok) return 1;
    return a->gmtoff == b->gmtoff && a->isdst == b->isdst && strcmp(a->abbr, b->abbr) == 0;
}

/* "Sun Mar 29 01:00:00 2020", as asctime(3) writes it without the newline. */
static void put_date(const struct tm *tm) {
    static const char wd[7][4] = {"Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"};
    static const char mo[12][4] = {"Jan", "Feb", "Mar", "Apr", "May", "Jun",
                                   "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"};
    const char *w = tm->tm_wday >= 0 && tm->tm_wday < 7 ? wd[tm->tm_wday] : "???";
    const char *m = tm->tm_mon >= 0 && tm->tm_mon < 12 ? mo[tm->tm_mon] : "???";
    printf("%s %s %2d %02d:%02d:%02d %lld", w, m, tm->tm_mday, tm->tm_hour, tm->tm_min, tm->tm_sec,
           (long long)tm->tm_year + 1900);
}

static int width;

/* Abbreviations already checked for this zone (the warning is given once per abbreviation). */
static char **seen;
static size_t nseen;

static void check_abbr(const char *zone, const char *abbr) {
    for (size_t i = 0; i < nseen; i++)
        if (strcmp(seen[i], abbr) == 0) return;
    char **n = realloc(seen, (nseen + 1) * sizeof *seen);
    char *copy = strdup(abbr);
    if (!n || !copy) {
        free(copy);
        if (n) seen = n;
        return;
    }
    seen = n;
    seen[nseen++] = copy;
    if (strlen(abbr) < 3) {
        fflush(stdout);
        fprintf(stderr, "%s: warning: zone \"%s\" abbreviation \"%s\" has fewer than 3 characters\n", progname, zone,
                abbr);
    }
}

static void forget_abbrs(void) {
    for (size_t i = 0; i < nseen; i++) free(seen[i]);
    nseen = 0;
}

/* One -v line for time T. */
static void verbose_line(const char *zone, time_t t) {
    struct tm ut;
    struct state s;
    printf("%-*s  ", width, zone);
    if (gmtime_r(&t, &ut)) {
        put_date(&ut);
        fputs(" UT", stdout);
    } else {
        printf("%" PRId64, (int64_t)t);
    }
    fputs(" = ", stdout);
    sample(t, &s);
    if (!s.ok) {
        puts("NULL");
        return;
    }
    put_date(&s.tm);
    if (*s.abbr) printf(" %s", s.abbr);
    printf(" isdst=%d gmtoff=%ld\n", s.isdst, s.gmtoff);
    check_abbr(zone, s.abbr);
}

static void put_offset(long off) {
    long a = off < 0 ? -off : off;
    printf("%c%02ld", off < 0 ? '-' : '+', a / 3600);
    if (a % 3600) {
        printf("%02ld", a / 60 % 60);
        if (a % 60) printf("%02ld", a % 60);
    }
}

/* The offset column of -i: "-00" (local time unspecified) is shown as itself. */
static void put_brief_offset(const struct state *s) {
    if (s->gmtoff == 0 && strcmp(s->abbr, "-00") == 0) fputs("-00", stdout);
    else put_offset(s->gmtoff);
}

/* The abbreviation column of -i: left empty when it only repeats the offset. */
static void put_brief_abbr(const struct state *s) {
    char off[32];
    long a = s->gmtoff < 0 ? -s->gmtoff : s->gmtoff;
    int n = snprintf(off, sizeof off, "%c%02ld", s->gmtoff < 0 ? '-' : '+', a / 3600);
    if (a % 3600) {
        n += snprintf(off + n, sizeof off - (size_t)n, "%02ld", a / 60 % 60);
        if (a % 60) snprintf(off + n, sizeof off - (size_t)n, "%02ld", a % 60);
    }
    int show = strcmp(off, s->abbr) != 0 && !(s->gmtoff == 0 && strcmp(s->abbr, "-00") == 0);
    if (show || s->isdst) printf("\t%s", !show ? "" : *s->abbr ? s->abbr : "\"\"");
    if (s->isdst) fputs("\t1", stdout);
    putchar('\n');
}

static void brief_line(const struct state *s) {
    printf("%04lld-%02d-%02d\t%02d", (long long)s->tm.tm_year + 1900, s->tm.tm_mon + 1, s->tm.tm_mday, s->tm.tm_hour);
    if (s->tm.tm_min || s->tm.tm_sec) {
        printf(":%02d", s->tm.tm_min);
        if (s->tm.tm_sec) printf(":%02d", s->tm.tm_sec);
    }
    putchar('\t');
    put_brief_offset(s);
    put_brief_abbr(s);
}

/* Seconds of 00:00 UT on January 1 of year Y. */
static time_t year_start(long long y) {
    /* Days from 1970-01-01 to y-01-01, proleptic Gregorian, counted in 400-year eras from March. */
    long long yy = y - 1;
    long long era = (yy >= 0 ? yy : yy - 399) / 400;
    long long yoe = yy - era * 400;
    long long doe = yoe * 365 + yoe / 4 - yoe / 100 + 306; /* January 1 is day 306 of the March-based year */
    return (time_t)((era * 146097 + doe - 719468) * 86400);
}

static int parse_range(const char *arg, int years, time_t *lo, time_t *hi) {
    char *end;
    errno = 0;
    long long a = strtoll(arg, &end, 10);
    if (end == arg || errno) return -1;
    long long b;
    int two = 0;
    if (*end == ',') {
        const char *p = end + 1;
        errno = 0;
        b = strtoll(p, &end, 10);
        if (end == p || errno) return -1;
        two = 1;
    }
    if (*end) return -1;
    if (years) {
        long long limit = 100000000000LL;
        if ((two && (a < -limit || a > limit)) || (two ? b : a) < -limit || (two ? b : a) > limit) return -1;
        if (two) {
            *lo = year_start(a);
            *hi = year_start(b);
        } else {
            *hi = year_start(a);
        }
    } else {
        if (two) {
            *lo = (time_t)a;
            *hi = (time_t)b;
        } else {
            *hi = (time_t)a;
        }
    }
    return 0;
}

static void show_zone(const char *zone, int mode, time_t lo, time_t hi) {
    if (setenv("TZ", zone, 1) != 0) {
        fprintf(stderr, "%s: %s\n", progname, strerror(errno));
        exit(1);
    }
    tzset();
    forget_abbrs();
    if (mode == 0) {
        time_t now = time(NULL);
        struct state s;
        sample(now, &s);
        printf("%-*s  ", width, zone);
        if (s.ok) {
            put_date(&s.tm);
            if (*s.abbr) printf(" %s", s.abbr);
            putchar('\n');
            check_abbr(zone, s.abbr);
        } else {
            printf("%" PRId64 " = NULL\n", (int64_t)now);
        }
        return;
    }
    if (mode == 'v') {
        verbose_line(zone, INT64_MIN);
        verbose_line(zone, INT64_MIN + 86400);
    }
    struct state cur, next;
    time_t t = lo;
    sample(t, &cur);
    if (mode == 'i') {
        printf("\nTZ=\"%s\"\n", zone);
        if (cur.ok) {
            fputs("-\t-\t", stdout);
            put_brief_offset(&cur);
            put_brief_abbr(&cur);
        }
    }
    while (t < hi) {
        time_t step = t > hi - 43200 ? hi : t + 43200;
        sample(step, &next);
        if (same(&cur, &next)) {
            t = step;
            continue;
        }
        /* Narrow to the first second that differs from cur. */
        time_t a = t, b = step;
        while (b - a > 1) {
            time_t mid = a + (b - a) / 2;
            struct state m;
            sample(mid, &m);
            if (same(&cur, &m)) a = mid;
            else b = mid;
        }
        sample(b, &next);
        if (b < hi) {
            if (mode == 'i') {
                if (next.ok) brief_line(&next);
            } else {
                verbose_line(zone, b - 1);
                verbose_line(zone, b);
            }
        }
        cur = next;
        t = b;
    }
    if (mode == 'v') {
        verbose_line(zone, INT64_MAX - 86400);
        verbose_line(zone, INT64_MAX);
    }
}

int main(int argc, char **argv) {
    int mode = 0;
    if (argc > 0 && argv[0]) progname = argv[0]; /* messages name the program as it was run */
    time_t lo = year_start(-500), hi = year_start(2500);
    int i = 1;
    for (; i < argc; i++) {
        const char *a = argv[i];
        if (strcmp(a, "--version") == 0) {
            puts("zdump (libniji) 2.39");
            return 0;
        }
        if (strcmp(a, "--help") == 0) usage(stdout, 0);
        if (strcmp(a, "--") == 0) {
            i++;
            break;
        }
        if (a[0] != '-' || a[1] == 0) break;
        for (const char *p = a + 1; *p; p++) {
            if (*p == 'v' || *p == 'V' || *p == 'i') {
                mode = *p == 'V' ? 'V' : *p;
            } else if (*p == 'c' || *p == 't') {
                const char *arg = p[1] ? p + 1 : (i + 1 < argc ? argv[++i] : NULL);
                if (!arg) usage(stderr, 1);
                if (parse_range(arg, *p == 'c', &lo, &hi)) {
                    fprintf(stderr, "%s: wild -%c argument %s\n", progname, *p, arg);
                    return 1;
                }
                break;
            } else {
                fprintf(stderr, "%s: invalid option -- '%c'\n", progname, *p);
                usage(stderr, 1);
            }
        }
    }
    width = 0;
    for (int j = i; j < argc; j++) {
        int n = (int)strlen(argv[j]);
        if (n > width) width = n;
    }
    for (int j = i; j < argc; j++) show_zone(argv[j], mode, lo, hi);
    if (fflush(stdout) != 0 || ferror(stdout)) {
        fprintf(stderr, "%s: Error writing to standard output\n", progname);
        return 1;
    }
    return 0;
}
