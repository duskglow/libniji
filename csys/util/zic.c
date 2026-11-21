/* zic: compile time-zone source into TZif files (spec §31.9, plan/utilities.md, story TZ-TOOLS).
 *
 * Written from the input language of zic(8) (Rule, Zone, Link, Leap and Expires lines), the TZif format of
 * RFC 9636 (versions 2 to 4) and black-box runs of the reference command on the shipped tzdata source; no
 * implementation of the tz tools was consulted. Built by `cargo xtask so` into target/niji/zic, shipped in
 * libc-bin as /usr/sbin/zic.
 *
 * Model. Each zone is a sequence of lines, each in force from the previous line's end (UNTIL) up to its own.
 * A line gives a standard offset and either standard time, a fixed amount of saved time, or a named rule set.
 * The engine walks the lines in order and produces the zone's transitions: one where a line starts (when the
 * local time type changes) and one at each rule transition inside the line. A local time type is the offset,
 * the DST flag, the abbreviation and (in fat output) the two indicators of how the transition time was given
 * (standard or UT, from the time's suffix). A transition that changes nothing a reader sees is dropped.
 *
 * Output (observed on the reference and fixed by RFC 9636): version 2 unless the footer needs version 3
 * (hours outside 0..24 in a rule time) or the leap table needs version 4; the footer is the POSIX TZ string
 * of the last line. `-b fat` (the default) writes a full version 1 block (32-bit transitions, a transition at
 * the lowest 32-bit time when earlier ones exist), the indicator arrays, and every transition of rule sets
 * that last forever up to 2037; `-b slim` writes an empty version 1 block, no indicators, and relies on the
 * footer once the last rule set starts.
 *
 * Errors in the source are reported as `"FILE", line N: MESSAGE` on standard error; any error means no file
 * is written and the exit status is 1. Input is untrusted: every count and time is bounded, nothing indexes
 * outside what was read, and a zone name may not leave the output directory.
 */
#define _GNU_SOURCE
#include <ctype.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <sys/stat.h>
#include <unistd.h>

typedef int64_t zt;

#define YEAR_LIMIT 2000000000LL /* years beyond this are refused: times stay far inside int64 */
#define MAX_EVENTS 1000000      /* transitions per zone */
#define FAT_LAST_YEAR 2037

static const char *progname = "zic";
static int errors;
static const char *cur_file = "";
static long cur_line;

static void fatal_oom(void) {
    fprintf(stderr, "%s: Memory exhausted\n", progname);
    exit(1);
}

static void *xrealloc(void *p, size_t n) {
    void *q = realloc(p, n ? n : 1);
    if (!q) fatal_oom();
    return q;
}

static char *xstrdup(const char *s) {
    char *d = strdup(s);
    if (!d) fatal_oom();
    return d;
}

static void error_at(const char *file, long line, const char *fmt, ...) __attribute__((format(printf, 3, 4)));
static void error_at(const char *file, long line, const char *fmt, ...) {
    va_list ap;
    fprintf(stderr, "\"%s\", line %ld: ", file, line);
    va_start(ap, fmt);
    vfprintf(stderr, fmt, ap);
    va_end(ap);
    fputc('\n', stderr);
    errors = 1;
}
#define ERR(...) error_at(cur_file, cur_line, __VA_ARGS__)

/* ---- civil time ---- */

static int is_leap(zt y) { return (y % 4 == 0 && y % 100 != 0) || y % 400 == 0; }

static const int month_days[12] = {31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31};

static int days_in_month(zt y, int m) { return m == 1 && is_leap(y) ? 29 : month_days[m]; }

/* Days from 1970-01-01 to y-m-d (m 0..11, d 1..31), proleptic Gregorian. */
static zt days_from_civil(zt y, int m, int d) {
    zt mm = m + 1;
    y -= mm <= 2;
    zt era = (y >= 0 ? y : y - 399) / 400;
    zt yoe = y - era * 400;
    zt doy = (153 * (mm + (mm > 2 ? -3 : 9)) + 2) / 5 + d - 1;
    zt doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    return era * 146097 + doe - 719468;
}

/* Year of a time, for choosing which rule years can matter. */
static zt year_of(zt t) {
    zt days = t / 86400 - (t % 86400 < 0);
    zt z = days + 719468;
    zt era = (z >= 0 ? z : z - 146096) / 146097;
    zt doe = z - era * 146097;
    zt yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    zt doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    zt mp = (5 * doy + 2) / 153;
    zt m = mp < 10 ? mp + 3 : mp - 9;
    return yoe + era * 400 + (m <= 2);
}

/* Weekday of a day number (0 = Sunday); 1970-01-01 was a Thursday. */
static int weekday(zt days) {
    zt w = (days + 4) % 7;
    return (int)(w < 0 ? w + 7 : w);
}

/* ---- words ---- */

static const char *const month_names[] = {"January", "February", "March",     "April",   "May",      "June",
                                          "July",    "August",   "September", "October", "November", "December",
                                          NULL};
static const char *const wday_names[] = {"Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday",
                                         "Saturday", NULL};

/* Index of WORD in TABLE: an exact match, or the one entry WORD is a case-insensitive prefix of; -1. */
static int lookup(const char *word, const char *const *table) {
    size_t n = strlen(word);
    int found = -1;
    if (n == 0) return -1;
    for (int i = 0; table[i]; i++) {
        if (strcasecmp(word, table[i]) == 0) return i;
        if (strncasecmp(word, table[i], n) == 0) {
            if (found >= 0) return -1;
            found = i;
        }
    }
    return found;
}

static int is_prefix_of(const char *word, const char *full, size_t min) {
    size_t n = strlen(word);
    return n >= min && n <= strlen(full) && strncasecmp(word, full, n) == 0;
}

/* A time of day or offset: [-]h[:mm[:ss[.frac]]] or "-" (zero). Sets *out in seconds. 0 on success. */
static int parse_hms(const char *s, zt *out) {
    int neg = 0;
    if (strcmp(s, "-") == 0) {
        *out = 0;
        return 0;
    }
    if (*s == '-') {
        neg = 1;
        s++;
    } else if (*s == '+') {
        s++;
    }
    if (!isdigit((unsigned char)*s)) return -1;
    zt h = 0, m = 0, sec = 0;
    while (isdigit((unsigned char)*s)) {
        h = h * 10 + (*s++ - '0');
        if (h > 1000000) return -1;
    }
    if (*s == ':') {
        s++;
        if (!isdigit((unsigned char)s[0])) return -1;
        m = s[0] - '0';
        s++;
        if (isdigit((unsigned char)*s)) m = m * 10 + (*s++ - '0');
        if (m > 59) return -1;
        if (*s == ':') {
            s++;
            if (!isdigit((unsigned char)s[0])) return -1;
            sec = s[0] - '0';
            s++;
            if (isdigit((unsigned char)*s)) sec = sec * 10 + (*s++ - '0');
            if (sec > 59) return -1;
            if (*s == '.') {
                /* Fractional seconds round to the nearest second, ties to even. */
                s++;
                if (!isdigit((unsigned char)*s)) return -1;
                int first = *s++ - '0';
                int rest = 0;
                while (isdigit((unsigned char)*s)) rest |= *s++ != '0';
                if (first > 5 || (first == 5 && (rest || (sec & 1)))) sec++;
            }
        }
    }
    if (*s) return -1;
    zt v = h * 3600 + m * 60 + sec;
    *out = neg ? -v : v;
    return 0;
}

/* A time with an optional one-letter suffix from SUFFIXES; *kind gets the letter or 0. */
static int parse_time_suffix(const char *s, const char *suffixes, zt *out, char *kind) {
    char buf[64];
    size_t n = strlen(s);
    *kind = 0;
    if (n == 0 || n >= sizeof buf) return -1;
    memcpy(buf, s, n + 1);
    char last = (char)tolower((unsigned char)buf[n - 1]);
    if (n > 1 && strchr(suffixes, last) && !isdigit((unsigned char)buf[n - 1])) {
        *kind = last;
        buf[n - 1] = 0;
    }
    return parse_hms(buf, out);
}

static int parse_year(const char *s, zt *y) {
    const char *p = s;
    if (*p == '-' || *p == '+') p++;
    if (!isdigit((unsigned char)*p)) return -1;
    zt v = 0;
    for (; *p; p++) {
        if (!isdigit((unsigned char)*p)) return -1;
        v = v * 10 + (*p - '0');
        if (v > YEAR_LIMIT) return -1;
    }
    *y = *s == '-' ? -v : v;
    return 0;
}

/* ---- the day of a rule or an UNTIL ---- */

enum { DAY_DOM, DAY_LAST, DAY_GE, DAY_LE };

struct dayspec {
    int kind, wday, dom;
};

static int parse_day(const char *s, int month, struct dayspec *d) {
    if (strncasecmp(s, "last", 4) == 0) {
        int w = lookup(s + 4, wday_names);
        if (w < 0) return -2;
        d->kind = DAY_LAST;
        d->wday = w;
        return 0;
    }
    const char *op = strstr(s, ">=");
    if (!op) op = strstr(s, "<=");
    if (op) {
        char w[32];
        size_t n = (size_t)(op - s);
        if (n == 0 || n >= sizeof w) return -2;
        memcpy(w, s, n);
        w[n] = 0;
        int wd = lookup(w, wday_names);
        if (wd < 0) return -2;
        zt dom;
        const char *q = op + 2;
        if (!isdigit((unsigned char)*q) || parse_year(q, &dom) || dom < 1 || dom > month_days[month] + (month == 1))
            return -1;
        d->kind = op[0] == '>' ? DAY_GE : DAY_LE;
        d->wday = wd;
        d->dom = (int)dom;
        return 0;
    }
    zt dom;
    if (!isdigit((unsigned char)*s) || parse_year(s, &dom) || dom < 1 || dom > month_days[month] + (month == 1))
        return -1;
    d->kind = DAY_DOM;
    d->dom = (int)dom;
    return 0;
}

/* Day number of the day DS names in year Y, month M; a day pushed into the next or previous month is kept
 * (zic(8) allows Sun>=29 style days to move across a month end). */
static zt resolve_day(zt y, int m, const struct dayspec *ds) {
    zt first = days_from_civil(y, m, 1);
    switch (ds->kind) {
    case DAY_DOM: return first + ds->dom - 1;
    case DAY_LAST: {
        zt last = first + days_in_month(y, m) - 1;
        return last - (weekday(last) - ds->wday + 7) % 7;
    }
    case DAY_GE: {
        zt base = first + ds->dom - 1;
        return base + (ds->wday - weekday(base) + 7) % 7;
    }
    default: {
        zt base = first + ds->dom - 1;
        return base - (weekday(base) - ds->wday + 7) % 7;
    }
    }
}

/* ---- the source tables ---- */

struct rule {
    char *name;
    zt from, to; /* INT64_MIN / INT64_MAX for min / max */
    int month;
    struct dayspec day;
    zt at;
    char at_kind; /* 'w', 's' or 'u' */
    zt save;
    int isdst;
    char *letters;
    const char *file;
    long line;
};

enum { RULES_NONE, RULES_FIXED, RULES_NAMED };

struct zline {
    zt stdoff;
    int rules_kind;
    zt save;
    int isdst;
    char *rules_name;
    struct rule **rules; /* the named set, resolved after reading */
    size_t nrules;
    char *format;
    int has_until;
    zt until_local; /* seconds of the local date-time */
    char until_kind;
    const char *file;
    long line;
};

struct zone {
    char *name;
    size_t first, count;
    const char *file;
    long line;
};

struct link {
    char *target, *name;
    const char *file;
    long line;
};

struct leap {
    zt t; /* UT of the leap second */
    int corr;
    int rolling;
};

static struct rule *rules;
static size_t nrules;
static struct zline *zlines;
static size_t nzlines;
static struct zone *zones;
static size_t nzones;
static struct link *links;
static size_t nlinks;
static struct leap *leaps;
static size_t nleaps;
static int have_expires;
static zt expires_at;
static int have_hash_expires;
static zt hash_expires;

static int fat = 1;

/* ---- fields ---- */

static int split_fields(char *line, char ***out, size_t *n) {
    static char **f;
    static size_t cap;
    size_t k = 0;
    char *r = line, *w = line;
    for (;;) {
        while (*r == ' ' || *r == '\t' || *r == '\f' || *r == '\v' || *r == '\r') r++;
        if (*r == 0 || *r == '#') break;
        char *start = w;
        while (*r && !(*r == ' ' || *r == '\t' || *r == '\f' || *r == '\v' || *r == '\r')) {
            if (*r == '#') break;
            if (*r == '"') {
                r++;
                while (*r && *r != '"') *w++ = *r++;
                if (*r != '"') return -1;
                r++;
            } else {
                *w++ = *r++;
            }
        }
        char stop = *r;
        if (stop) r++;
        *w++ = 0;
        if (k == cap) {
            cap = cap ? cap * 2 : 16;
            f = xrealloc(f, cap * sizeof *f);
        }
        f[k++] = start;
        if (stop == '#') break;
    }
    *out = f;
    *n = k;
    return 0;
}

static int check_format(const char *f) {
    int slash = strchr(f, '/') != NULL;
    if (!*f) return -1;
    for (const char *p = f; *p; p++) {
        if (*p == '%') {
            p++;
            if (*p == 's') {
                if (slash) return -1;
            } else if (*p != 'z' && *p != '%') {
                return -1;
            }
        }
    }
    return 0;
}

static int parse_rules_field(struct zline *z, const char *s) {
    z->rules_name = NULL;
    if (strcmp(s, "-") == 0 || *s == 0) {
        z->rules_kind = RULES_NONE;
        return 0;
    }
    if (isdigit((unsigned char)s[0]) || ((s[0] == '-' || s[0] == '+') && isdigit((unsigned char)s[1]))) {
        char kind;
        if (parse_time_suffix(s, "sd", &z->save, &kind)) return -1;
        z->rules_kind = RULES_FIXED;
        z->isdst = kind == 's' ? 0 : kind == 'd' ? 1 : z->save != 0;
        return 0;
    }
    z->rules_kind = RULES_NAMED;
    z->rules_name = xstrdup(s);
    return 0;
}

/* UNTIL fields f[0..n): year [month [day [time]]]. */
static int parse_until(struct zline *z, char **f, size_t n) {
    zt y;
    int m = 0;
    struct dayspec ds = {DAY_DOM, 0, 1};
    zt tod = 0;
    char kind = 'w';
    if (parse_year(f[0], &y)) {
        ERR("invalid year");
        return -1;
    }
    if (n > 1) {
        m = lookup(f[1], month_names);
        if (m < 0) {
            ERR("invalid month name");
            return -1;
        }
    }
    if (n > 2) {
        int r = parse_day(f[2], m, &ds);
        if (r) {
            ERR(r == -2 ? "invalid weekday name" : "invalid day of month");
            return -1;
        }
    }
    if (n > 3) {
        char k;
        if (parse_time_suffix(f[3], "wsugz", &tod, &k)) {
            ERR("invalid time of day");
            return -1;
        }
        if (k) kind = k == 'g' || k == 'z' ? 'u' : k;
    }
    z->has_until = 1;
    z->until_local = resolve_day(y, m, &ds) * 86400 + tod;
    z->until_kind = kind;
    return 0;
}

/* A zone or link name becomes a file under the output directory: it must be relative and hold no "." or ".."
 * component and no empty one, so that it names a file below that directory. Components are checked from the
 * left and the first fault is reported (observed wording and order). 1 if NAME is usable; else 0 after the
 * error. */
static int valid_zone_name(const char *name) {
    if (!*name) {
        ERR("empty file name");
        return 0;
    }
    if (*name == '/') {
        ERR("file name '%s' begins with '/'", name);
        return 0;
    }
    for (const char *p = name;;) {
        const char *e = strchr(p, '/');
        size_t n = e ? (size_t)(e - p) : strlen(p);
        if (n == 1 && p[0] == '.') {
            ERR("file name '%s' contains '.' component", name);
            return 0;
        }
        if (n == 2 && p[0] == '.' && p[1] == '.') {
            ERR("file name '%s' contains '..' component", name);
            return 0;
        }
        if (n == 0) {
            if (e) ERR("file name '%s' contains '//'", name);
            else ERR("file name '%s' ends with '/'", name);
            return 0;
        }
        if (!e) return 1;
        p = e + 1;
    }
}

static int expect_cont;

static void zone_line(char **f, size_t n, int cont) {
    struct zline z;
    memset(&z, 0, sizeof z);
    /* A line in error ends its zone: no continuation is expected after it (observed). */
    expect_cont = 0;
    z.file = cur_file;
    z.line = cur_line;
    if (parse_hms(f[0], &z.stdoff)) {
        ERR("invalid UT offset");
        return;
    }
    if (parse_rules_field(&z, f[1])) {
        ERR("invalid saved time");
        return;
    }
    if (check_format(f[2])) {
        ERR("invalid abbreviation format");
        return;
    }
    z.format = xstrdup(f[2]);
    if (!strchr(f[2], '%')) {
        /* A literal abbreviation (or pair): the warnings the reference gives, the line still compiles. */
        char *copy = xstrdup(f[2]);
        for (char *part = strtok(copy, "/"); part; part = strtok(NULL, "/")) {
            int posix = 1;
            for (const char *p = part; *p; p++)
                if (!isalnum((unsigned char)*p) && *p != '+' && *p != '-') posix = 0;
            if (strlen(part) > 6)
                fprintf(stderr, "warning: \"%s\", line %ld: time zone abbreviation has too many characters (%s)\n",
                        cur_file, cur_line, part);
            else if (!posix)
                fprintf(stderr, "warning: \"%s\", line %ld: time zone abbreviation differs from POSIX standard (%s)\n",
                        cur_file, cur_line, part);
        }
        free(copy);
    }
    if (n > 3 && parse_until(&z, f + 3, n - 3)) return;
    if (cont && nzlines > 0) {
        struct zline *prev = &zlines[nzlines - 1];
        /* Compare the ends in UT as far as the line's own fields allow. */
        if (z.has_until && prev->has_until) {
            zt a = prev->until_local - (prev->until_kind == 'u' ? 0 : prev->stdoff);
            zt b = z.until_local - (z.until_kind == 'u' ? 0 : z.stdoff);
            if (b <= a) {
                ERR("Zone continuation line end time is not after end time of previous line");
                return;
            }
        }
    }
    zlines = xrealloc(zlines, (nzlines + 1) * sizeof *zlines);
    zlines[nzlines++] = z;
    zones[nzones - 1].count++;
    expect_cont = z.has_until;
}

static void rule_line(char **f, size_t n) {
    struct rule r;
    memset(&r, 0, sizeof r);
    if (n != 10) {
        ERR("wrong number of fields on Rule line");
        return;
    }
    r.file = cur_file;
    r.line = cur_line;
    if (is_prefix_of(f[2], "minimum", 2)) r.from = INT64_MIN;
    else if (is_prefix_of(f[2], "maximum", 2)) {
        ERR("invalid starting year");
        return;
    } else if (parse_year(f[2], &r.from)) {
        ERR("invalid starting year");
        return;
    }
    if (is_prefix_of(f[3], "only", 1)) r.to = r.from;
    else if (is_prefix_of(f[3], "maximum", 2)) r.to = INT64_MAX;
    else if (is_prefix_of(f[3], "minimum", 2)) r.to = INT64_MIN;
    else if (parse_year(f[3], &r.to)) {
        ERR("invalid ending year");
        return;
    }
    if (r.to == INT64_MIN && r.from != INT64_MIN) {
        ERR("invalid ending year");
        return;
    }
    if (r.from > r.to) {
        ERR("starting year greater than ending year");
        return;
    }
    if (strcmp(f[4], "-") != 0 && *f[4]) {
        ERR("year type \"%s\" is not supported", f[4]);
        return;
    }
    r.month = lookup(f[5], month_names);
    if (r.month < 0) {
        ERR("invalid month name");
        return;
    }
    int d = parse_day(f[6], r.month, &r.day);
    if (d) {
        ERR(d == -2 ? "invalid weekday name" : "invalid day of month");
        return;
    }
    char k;
    if (parse_time_suffix(f[7], "wsugz", &r.at, &k)) {
        ERR("invalid time of day");
        return;
    }
    r.at_kind = !k ? 'w' : k == 'g' || k == 'z' ? 'u' : k;
    if (parse_time_suffix(f[8], "sd", &r.save, &k)) {
        ERR("invalid saved time");
        return;
    }
    r.isdst = k == 's' ? 0 : k == 'd' ? 1 : r.save != 0;
    r.name = xstrdup(f[1]);
    r.letters = xstrdup(strcmp(f[9], "-") == 0 ? "" : f[9]);
    rules = xrealloc(rules, (nrules + 1) * sizeof *rules);
    rules[nrules++] = r;
}

static void zone_start(char **f, size_t n) {
    if (n < 5 || n > 9) {
        ERR("wrong number of fields on Zone line");
        return;
    }
    if (!valid_zone_name(f[1])) return;
    for (size_t i = 0; i < nzones; i++) {
        if (strcmp(zones[i].name, f[1]) == 0) {
            ERR("duplicate zone name %s (file \"%s\", line %ld)", f[1], zones[i].file, zones[i].line);
            return;
        }
    }
    zones = xrealloc(zones, (nzones + 1) * sizeof *zones);
    zones[nzones].name = xstrdup(f[1]);
    zones[nzones].first = nzlines;
    zones[nzones].count = 0;
    zones[nzones].file = cur_file;
    zones[nzones].line = cur_line;
    nzones++;
    /* A first line in error leaves the zone empty; the error stops the run before any zone is written. */
    zone_line(f + 2, n - 2, 0);
}

static void link_line(char **f, size_t n) {
    if (n != 3) {
        ERR("wrong number of fields on Link line");
        return;
    }
    if (!*f[1]) {
        ERR("blank FROM field on Link line");
        return;
    }
    if (!valid_zone_name(f[2])) return;
    links = xrealloc(links, (nlinks + 1) * sizeof *links);
    links[nlinks].target = xstrdup(f[1]);
    links[nlinks].name = xstrdup(f[2]);
    links[nlinks].file = cur_file;
    links[nlinks].line = cur_line;
    nlinks++;
}

/* Leap YEAR MONTH DAY HH:MM:SS CORR R/S, and Expires YEAR MONTH DAY HH:MM:SS (the -L file only). */
static int leap_time(char **f, zt *t) {
    zt y, tod;
    int m = lookup(f[1], month_names);
    struct dayspec ds;
    if (parse_year(f[0], &y)) {
        ERR("invalid leaping year");
        return -1;
    }
    if (m < 0) {
        ERR("invalid month name");
        return -1;
    }
    int d = parse_day(f[2], m, &ds);
    if (d || ds.kind != DAY_DOM) {
        ERR("invalid day of month");
        return -1;
    }
    /* A leap second is written 23:59:60; the time is the instant after it, less one. */
    char buf[64];
    size_t len = strlen(f[3]);
    if (len >= sizeof buf) {
        ERR("invalid time of day");
        return -1;
    }
    memcpy(buf, f[3], len + 1);
    int sixty = 0;
    if (len >= 3 && strcmp(buf + len - 3, ":60") == 0) {
        sixty = 1;
        buf[len - 1] = '9';
        buf[len - 2] = '5';
    }
    if (parse_hms(buf, &tod) || tod < 0) {
        ERR("invalid time of day");
        return -1;
    }
    *t = resolve_day(y, m, &ds) * 86400 + tod + sixty;
    return 0;
}

static void leap_line(char **f, size_t n) {
    if (n != 7) {
        ERR("wrong number of fields on Leap line");
        return;
    }
    struct leap l;
    if (leap_time(f + 1, &l.t)) return;
    if (strcmp(f[5], "+") == 0) l.corr = 1;
    else if (strcmp(f[5], "-") == 0) l.corr = -1;
    else {
        ERR("invalid CORRECTION field on Leap line");
        return;
    }
    if (is_prefix_of(f[6], "Rolling", 1)) l.rolling = 1;
    else if (is_prefix_of(f[6], "Stationary", 1)) l.rolling = 0;
    else {
        ERR("invalid Rolling/Stationary field on Leap line");
        return;
    }
    /* l.t is the instant the correction takes effect: the end of an inserted 23:59:60, or of a skipped second. */
    leaps = xrealloc(leaps, (nleaps + 1) * sizeof *leaps);
    leaps[nleaps++] = l;
}

static void expires_line(char **f, size_t n) {
    if (n != 5) {
        ERR("wrong number of fields on Expires line");
        return;
    }
    if (have_expires) {
        ERR("multiple Expires lines");
        return;
    }
    if (leap_time(f + 1, &expires_at)) return;
    have_expires = 1;
}

static int read_file(const char *path, int leapfile) {
    FILE *fp = strcmp(path, "-") == 0 ? stdin : fopen(path, "r");
    if (!fp) {
        fprintf(stderr, "%s: Can't open %s: %s\n", progname, path, strerror(errno));
        return -1;
    }
    cur_file = xstrdup(path);
    cur_line = 0;
    char *line = NULL;
    size_t cap = 0;
    ssize_t len;
    expect_cont = 0;
    while ((len = getline(&line, &cap, fp)) >= 0) {
        cur_line++;
        if (len > 0 && line[len - 1] == '\n') line[--len] = 0;
        if (memchr(line, 0, (size_t)len)) {
            ERR("NUL input byte");
            continue;
        }
        if (leapfile && strncmp(line, "#expires", 8) == 0 && (line[8] == ' ' || line[8] == '\t')) {
            /* The obsolescent comment form of the expiry: seconds since 1970. */
            char *end;
            errno = 0;
            long long v = strtoll(line + 9, &end, 10);
            if (!errno && end != line + 9) {
                hash_expires = v;
                have_hash_expires = 1;
            }
        }
        char **f;
        size_t n;
        if (split_fields(line, &f, &n)) {
            ERR("Odd number of quotation marks");
            continue;
        }
        if (n == 0) continue;
        if (expect_cont) {
            if (n < 3 || n > 7) {
                ERR("wrong number of fields on Zone continuation line");
                expect_cont = 0;
                continue;
            }
            zone_line(f, n, 1);
            continue;
        }
        const char *kw = f[0];
        if (leapfile) {
            if (is_prefix_of(kw, "Leap", 1)) leap_line(f, n);
            else if (is_prefix_of(kw, "Expires", 1)) expires_line(f, n);
            else ERR("input line of unknown type");
        } else if (strcasecmp(kw, "L") == 0 || is_prefix_of(kw, "Link", 2)) {
            link_line(f, n);
        } else if (is_prefix_of(kw, "Rule", 1)) {
            rule_line(f, n);
        } else if (is_prefix_of(kw, "Zone", 1)) {
            zone_start(f, n);
        } else {
            ERR("input line of unknown type");
        }
    }
    if (leapfile && have_hash_expires && !have_expires) {
        fprintf(stderr, "warning: \"%s\", line %ld: \"#expires\" is obsolescent; use \"Expires\"\n", cur_file,
                cur_line + 1);
        have_expires = 1;
        expires_at = hash_expires;
    }
    if (expect_cont) {
        cur_line++;
        ERR("expected continuation line not found");
        expect_cont = 0;
    }
    free(line);
    if (ferror(fp)) {
        fprintf(stderr, "%s: Error reading %s\n", progname, path);
        errors = 1;
    }
    if (fp != stdin) fclose(fp);
    return 0;
}

/* ---- the engine ---- */

struct ttype {
    zt utoff;
    int isdst;
    char *abbr;
    int isstd, isut;
};

struct event {
    zt at;
    struct ttype t;
};

static struct event *ev;
static size_t nev, evcap;
static int too_many;

static void push_event(zt at, const struct ttype *t) {
    if (nev >= MAX_EVENTS) {
        too_many = 1;
        return;
    }
    if (nev == evcap) {
        evcap = evcap ? evcap * 2 : 256;
        ev = xrealloc(ev, evcap * sizeof *ev);
    }
    ev[nev].at = at;
    ev[nev].t = *t;
    nev++;
}

/* The abbreviation of FORMAT for the given letters, DST flag and total offset. */
static char *abbreviation(const char *format, const char *letters, int isdst, zt utoff) {
    char out[256];
    size_t k = 0;
    const char *f = format;
    const char *slash = strchr(format, '/');
    size_t flen = strlen(format);
    if (slash) {
        if (isdst) f = slash + 1;
        else flen = (size_t)(slash - format);
        if (isdst) flen = strlen(f);
    }
    for (size_t i = 0; i < flen && k < sizeof out - 16; i++) {
        if (f[i] != '%') {
            out[k++] = f[i];
            continue;
        }
        i++;
        if (f[i] == 's') {
            for (const char *l = letters; *l && k < sizeof out - 16; l++) out[k++] = *l;
        } else if (f[i] == 'z') {
            zt a = utoff < 0 ? -utoff : utoff;
            int hh = (int)(a / 3600), mm = (int)(a / 60 % 60), ss = (int)(a % 60);
            k += (size_t)(ss ? snprintf(out + k, 16, "%c%02d%02d%02d", utoff < 0 ? '-' : '+', hh, mm, ss)
                          : mm ? snprintf(out + k, 16, "%c%02d%02d", utoff < 0 ? '-' : '+', hh, mm)
                               : snprintf(out + k, 16, "%c%02d", utoff < 0 ? '-' : '+', hh));
        } else {
            out[k++] = '%';
        }
    }
    out[k] = 0;
    return xstrdup(out);
}

/* The first rule transition of a named set that has no saved time gives the letters of standard time
 * before any rule has taken effect. */
static const char *initial_letters(const struct zline *z) {
    const struct rule *best = NULL;
    for (size_t i = 0; i < z->nrules; i++) {
        const struct rule *r = z->rules[i];
        if (r->save != 0) continue;
        if (!best || r->from < best->from || (r->from == best->from && r->month < best->month)) best = r;
    }
    return best ? best->letters : "";
}

struct cand {
    zt key, local, year;
    const struct rule *r;
};

static int cand_cmp(const void *a, const void *b) {
    const struct cand *x = a, *y = b;
    if (x->key != y->key) return x->key < y->key ? -1 : 1;
    return x->r < y->r ? -1 : x->r > y->r;
}

static struct cand *cands;
static size_t ncands, candcap;

/* Rule transitions of line Z for the years LO..HI, in time order. */
static void collect(const struct zline *z, zt lo, zt hi) {
    ncands = 0;
    for (size_t i = 0; i < z->nrules; i++) {
        const struct rule *r = z->rules[i];
        zt a = r->from > lo ? r->from : lo;
        zt b = r->to < hi ? r->to : hi;
        for (zt y = a; y <= b; y++) {
            if (ncands >= MAX_EVENTS) {
                too_many = 1;
                return;
            }
            if (ncands == candcap) {
                candcap = candcap ? candcap * 2 : 256;
                cands = xrealloc(cands, candcap * sizeof *cands);
            }
            zt local = resolve_day(y, r->month, &r->day) * 86400 + r->at;
            cands[ncands].local = local;
            cands[ncands].key = local - (r->at_kind == 'u' ? 0 : z->stdoff);
            cands[ncands].r = r;
            cands[ncands].year = y;
            ncands++;
        }
    }
    qsort(cands, ncands, sizeof *cands, cand_cmp);
}

static zt rule_ut(const struct zline *z, const struct cand *c, zt save) {
    switch (c->r->at_kind) {
    case 'u': return c->local;
    case 's': return c->local - z->stdoff;
    default: return c->local - z->stdoff - save;
    }
}

static zt until_ut(const struct zline *z, zt save) {
    switch (z->until_kind) {
    case 'u': return z->until_local;
    case 's': return z->until_local - z->stdoff;
    default: return z->until_local - z->stdoff - save;
    }
}

static int has_forever_rule(const struct zline *z) {
    for (size_t i = 0; i < z->nrules; i++)
        if (z->rules[i]->to == INT64_MAX) return 1;
    return 0;
}

static zt last_finite_year(const struct zline *z) {
    zt y = INT64_MIN;
    for (size_t i = 0; i < z->nrules; i++) {
        const struct rule *r = z->rules[i];
        if (r->from != INT64_MIN && r->from > y) y = r->from;
        if (r->to != INT64_MAX && r->to != INT64_MIN && r->to > y) y = r->to;
    }
    return y;
}

static zt first_finite_year(const struct zline *z) {
    zt y = INT64_MAX;
    for (size_t i = 0; i < z->nrules; i++) {
        const struct rule *r = z->rules[i];
        if (r->from != INT64_MIN && r->from < y) y = r->from;
        if (r->to != INT64_MAX && r->to != INT64_MIN && r->to < y) y = r->to;
    }
    return y;
}

static struct ttype initial_type;
static int have_last_start; /* the zone has more than one line; the last starts at last_start */
static zt last_start;

/* The first time each rule of the last line that lasts forever takes effect in that line. */
#define MAX_FIRING 16
static const struct rule *firing_rule[MAX_FIRING];
static zt firing_at[MAX_FIRING];
static size_t nfiring;
static zt last_finite_at; /* the last transition of the last line made by a rule that ends */

static void note_firing(const struct rule *r, zt at) {
    if (r->to != INT64_MAX) return;
    for (size_t i = 0; i < nfiring; i++)
        if (firing_rule[i] == r) return;
    if (nfiring == MAX_FIRING) return;
    firing_rule[nfiring] = r;
    firing_at[nfiring++] = at;
}

/* Run zone ZN: fills ev[] with the transitions (time order) and initial_type with the type before them. */
static void run_zone(const struct zone *zn) {
    nev = 0;
    too_many = 0;
    have_last_start = 0;
    nfiring = 0;
    last_finite_at = INT64_MIN;
    int have_start = 0; /* a line before this one ended */
    zt start = 0;
    char prev_until_kind = 'w';
    zt prev_stdoff = 0, prev_utoff = 0;
    for (size_t li = 0; li < zn->count; li++) {
        const struct zline *z = &zlines[zn->first + li];
        int last = li + 1 == zn->count;
        if (last && have_start) {
            have_last_start = 1;
            last_start = start;
        }
        struct ttype t;
        memset(&t, 0, sizeof t);
        t.isstd = prev_until_kind == 's' || prev_until_kind == 'u';
        t.isut = prev_until_kind == 'u';
        if (z->rules_kind != RULES_NAMED) {
            zt save = z->rules_kind == RULES_FIXED ? z->save : 0;
            t.utoff = z->stdoff + save;
            t.isdst = z->rules_kind == RULES_FIXED ? z->isdst : 0;
            t.abbr = abbreviation(z->format, "", t.isdst, t.utoff);
            if (have_start) push_event(start, &t);
            else {
                initial_type = t;
                initial_type.isstd = initial_type.isut = 0;
            }
            if (!last) {
                start = until_ut(z, save);
                have_start = 1;
            }
            prev_until_kind = z->until_kind;
            prev_stdoff = z->stdoff;
            prev_utoff = t.utoff;
            continue;
        }
        /* A named rule set: the years that can matter to this line. */
        zt lo, hi;
        /* From the rule set's first year: a rule from long before the line can still be in force at its start. */
        zt f = first_finite_year(z);
        lo = f == INT64_MAX ? 1900 : f - 1;
        if (have_start && year_of(start) - 1 < lo) lo = year_of(start) - 1;
        if (!last) hi = year_of(z->until_local) + 1;
        else {
            hi = last_finite_year(z);
            if (hi == INT64_MIN) hi = lo;
            if (has_forever_rule(z) && fat && hi < FAT_LAST_YEAR) hi = FAT_LAST_YEAR;
            if (have_start && year_of(start) > hi) hi = year_of(start);
            if (!fat) hi += 1;
        }
        if (lo < -YEAR_LIMIT) lo = -YEAR_LIMIT;
        if (hi > YEAR_LIMIT) hi = YEAR_LIMIT;
        if (hi - lo > MAX_EVENTS) {
            too_many = 1;
            return;
        }
        collect(z, lo, hi);
        if (too_many) return;
        zt save = 0;
        int isdst = 0;
        const char *letters = initial_letters(z);
        size_t ci = 0;
        /* The state at the line's start: the last rule transition at or before it. */
        if (have_start) {
            for (; ci < ncands; ci++) {
                /* Judged on the clock that was running before the line started: a rule taking effect at the
                 * local time the previous line ends at takes effect at the start. */
                const struct cand *c = &cands[ci];
                zt at = c->r->at_kind == 'u'   ? c->local
                        : c->r->at_kind == 's' ? c->local - prev_stdoff
                                               : c->local - prev_utoff;
                if (at > start) break;
                zt here = rule_ut(z, c, save);
                save = cands[ci].r->save;
                isdst = cands[ci].r->isdst;
                letters = cands[ci].r->letters;
                if (last && (at == start || here == start)) note_firing(cands[ci].r, start);
                if (at == start || here == start) {
                    /* A rule transition at the line's start, by either clock, gives the start its time
                     * indicators. */
                    t.isstd = cands[ci].r->at_kind == 's' || cands[ci].r->at_kind == 'u';
                    t.isut = cands[ci].r->at_kind == 'u';
                }
            }
        }
        t.utoff = z->stdoff + save;
        t.isdst = isdst;
        t.abbr = abbreviation(z->format, letters, isdst, t.utoff);
        if (have_start) push_event(start, &t);
        else {
            initial_type = t;
            initial_type.isstd = initial_type.isut = 0;
        }
        for (; ci < ncands; ci++) {
            const struct rule *r = cands[ci].r;
            zt at = rule_ut(z, &cands[ci], save);
            if (!last && at >= until_ut(z, save)) break;
            if (have_start && at <= start) continue;
            save = r->save;
            isdst = r->isdst;
            letters = r->letters;
            struct ttype u;
            u.utoff = z->stdoff + save;
            u.isdst = isdst;
            u.abbr = abbreviation(z->format, letters, isdst, u.utoff);
            u.isstd = r->at_kind == 's' || r->at_kind == 'u';
            u.isut = r->at_kind == 'u';
            push_event(at, &u);
            if (too_many) return;
            if (last) note_firing(r, at);
            if (last && r->to != INT64_MAX) last_finite_at = at;
        }
        if (!last) {
            start = until_ut(z, save);
            have_start = 1;
        }
        prev_until_kind = z->until_kind;
        prev_stdoff = z->stdoff;
        prev_utoff = z->stdoff + save;
    }
}

static int same_visible(const struct ttype *a, const struct ttype *b) {
    return a->utoff == b->utoff && a->isdst == b->isdst && strcmp(a->abbr, b->abbr) == 0;
}

/* Sort is already by time per line; lines follow each other. Drop the transitions that change nothing and
 * the ones a later transition at the same or an earlier time overrides. */
static void tidy_events(void) {
    size_t k = 0;
    for (size_t i = 0; i < nev; i++) {
        while (k > 0 && ev[k - 1].at >= ev[i].at) k--;
        const struct ttype *prev = k ? &ev[k - 1].t : &initial_type;
        if (same_visible(prev, &ev[i].t)) continue;
        ev[k++] = ev[i];
    }
    nev = k;
}

/* ---- the footer ---- */

static void put_offset(char *buf, size_t cap, zt off) {
    /* POSIX offsets are west-positive: the negation of the UT offset. */
    zt v = -off;
    const char *sign = v < 0 ? "-" : "";
    zt a = v < 0 ? -v : v;
    zt hh = a / 3600, mm = a / 60 % 60, ss = a % 60;
    if (ss) snprintf(buf, cap, "%s%lld:%02lld:%02lld", sign, (long long)hh, (long long)mm, (long long)ss);
    else if (mm) snprintf(buf, cap, "%s%lld:%02lld", sign, (long long)hh, (long long)mm);
    else snprintf(buf, cap, "%s%lld", sign, (long long)hh);
}

static void put_abbr(char *buf, size_t cap, const char *abbr) {
    int plain = *abbr != 0;
    for (const char *p = abbr; *p; p++)
        if (!isalpha((unsigned char)*p)) plain = 0;
    snprintf(buf, cap, plain ? "%s" : "<%s>", abbr);
}

static int footer_version; /* 3 when a rule time needs the version 3 range */

/* ",date[/time]" of rule R whose wall clock time is WALL seconds past the local midnight of its day. */
static int put_rule_date(char *buf, size_t cap, const struct rule *r, zt wall) {
    char date[32], tm[48];
    int m = r->month + 1;
    zt shift = 0;
    switch (r->day.kind) {
    case DAY_LAST: snprintf(date, sizeof date, "M%d.5.%d", m, r->day.wday); break;
    case DAY_GE:
    case DAY_LE: {
        int dom = r->day.kind == DAY_GE ? r->day.dom : r->day.dom - 6;
        int wd = r->day.wday;
        if (r->day.kind == DAY_LE && dom < 1) return -1;
        /* Week w covers days 7w-6..7w; another first day is the same weekday rule for an earlier weekday,
         * shifted by whole days. */
        int off = (dom - 1) % 7;
        if (off) {
            wd = (wd - off + 7) % 7;
            shift = (zt)off * 86400;
            dom -= off;
        }
        if (dom + 6 > month_days[r->month] && r->day.kind == DAY_GE && dom > 22) return -1;
        snprintf(date, sizeof date, "M%d.%d.%d", m, (dom - 1) / 7 + 1, wd);
        break;
    }
    default: {
        if (r->month == 1 && r->day.dom == 29) return -1;
        zt doy = days_from_civil(2001, r->month, r->day.dom) - days_from_civil(2001, 0, 1) + 1;
        snprintf(date, sizeof date, "J%lld", (long long)doy);
        break;
    }
    }
    wall += shift;
    tm[0] = 0;
    if (wall != 7200) {
        zt a = wall < 0 ? -wall : wall;
        zt hh = a / 3600, mm = a / 60 % 60, ss = a % 60;
        const char *sign = wall < 0 ? "-" : "";
        if (ss) snprintf(tm, sizeof tm, "/%s%lld:%02lld:%02lld", sign, (long long)hh, (long long)mm, (long long)ss);
        else if (mm) snprintf(tm, sizeof tm, "/%s%lld:%02lld", sign, (long long)hh, (long long)mm);
        else snprintf(tm, sizeof tm, "/%s%lld", sign, (long long)hh);
    }
    if (shift || wall < 0 || wall > 24 * 3600) footer_version = 3;
    snprintf(buf, cap, ",%s%s", date, tm);
    return 0;
}

/* The TZ string of the zone's last line; "" when it cannot be written. */
static void footer(const struct zone *zn, char *out, size_t cap) {
    const struct zline *z = &zlines[zn->first + zn->count - 1];
    char a[300], b[300], o1[48], o2[48], d1[96], d2[96];
    footer_version = 2;
    out[0] = 0;
    const struct rule *dst = NULL, *std = NULL;
    int nforever = 0;
    if (z->rules_kind == RULES_NAMED) {
        for (size_t i = 0; i < z->nrules; i++) {
            const struct rule *r = z->rules[i];
            if (r->to != INT64_MAX) continue;
            nforever++;
            if (r->isdst) dst = r;
            else std = r;
        }
    }
    if (nforever == 0) {
        /* The type in force after the last transition. */
        const struct ttype *t = nev ? &ev[nev - 1].t : &initial_type;
        put_abbr(a, sizeof a, t->abbr);
        put_offset(o1, sizeof o1, t->utoff);
        if (!t->isdst) {
            snprintf(out, cap, "%s%s", a, o1);
        } else {
            /* Daylight saving time all year: a DST period covering the whole year. */
            const char *s = "";
            char *sa = abbreviation(z->format, s, 0, z->stdoff);
            put_abbr(b, sizeof b, sa);
            free(sa);
            put_offset(o2, sizeof o2, z->stdoff);
            char o3[48];
            put_offset(o3, sizeof o3, t->utoff);
            snprintf(out, cap, "%s%s%s%s,0/0,J365/%lld", b, o2, a, t->utoff - z->stdoff == 3600 ? "" : o3,
                     (long long)(25 + (t->utoff - z->stdoff) / 3600));
            footer_version = 3;
        }
        return;
    }
    if (nforever != 2 || !dst || !std) return;
    char *sa = abbreviation(z->format, std->letters, 0, z->stdoff);
    char *da = abbreviation(z->format, dst->letters, 1, z->stdoff + dst->save);
    put_abbr(a, sizeof a, sa);
    put_abbr(b, sizeof b, da);
    free(sa);
    free(da);
    put_offset(o1, sizeof o1, z->stdoff);
    o2[0] = 0;
    if (dst->save != 3600) put_offset(o2, sizeof o2, z->stdoff + dst->save);
    zt w1 = dst->at + (dst->at_kind == 'u' ? z->stdoff + std->save : dst->at_kind == 's' ? std->save : 0);
    zt w2 = std->at + (std->at_kind == 'u' ? z->stdoff + dst->save : std->at_kind == 's' ? dst->save : 0);
    if (put_rule_date(d1, sizeof d1, dst, w1) || put_rule_date(d2, sizeof d2, std, w2)) {
        footer_version = 2;
        return;
    }
    snprintf(out, cap, "%s%s%s%s%s%s", a, o1, b, o2, d1, d2);
}

/* ---- the writer ---- */

struct buf {
    unsigned char *p;
    size_t n, cap;
};

static void put(struct buf *b, const void *data, size_t n) {
    if (b->n + n > b->cap) {
        b->cap = (b->n + n) * 2 + 256;
        b->p = xrealloc(b->p, b->cap);
    }
    memcpy(b->p + b->n, data, n);
    b->n += n;
}

static void put32(struct buf *b, int64_t v) {
    unsigned char c[4] = {(unsigned char)(v >> 24), (unsigned char)(v >> 16), (unsigned char)(v >> 8),
                          (unsigned char)v};
    put(b, c, 4);
}

static void put64(struct buf *b, int64_t v) {
    put32(b, (int64_t)((uint64_t)v >> 32));
    put32(b, v);
}

static struct ttype *types;
static size_t ntypes;
static int *tidx; /* per event */

static int type_eq(const struct ttype *a, const struct ttype *b) {
    if (!same_visible(a, b)) return 0;
    return !fat || (a->isstd == b->isstd && a->isut == b->isut);
}

static int type_of(const struct ttype *t) {
    for (size_t i = 0; i < ntypes; i++)
        if (type_eq(&types[i], t)) return (int)i;
    types = xrealloc(types, (ntypes + 1) * sizeof *types);
    types[ntypes] = *t;
    return (int)ntypes++;
}

/* Leap seconds as written: the time each takes effect in the file's (leap-counting) clock, and the total. */
static zt leap_shift(zt t) {
    zt c = 0;
    for (size_t i = 0; i < nleaps; i++)
        if (leaps[i].t < t) c += leaps[i].corr;
    return c;
}

static void write_block(struct buf *b, char version, int v1, const char *abbrs, size_t nabbr,
                        const int *abbr_at) {
    int wide = !v1;
    size_t first = 0, count = nev;
    int pseudo = -1;
    if (v1) {
        /* 32-bit times: the transitions in range, after one at the lowest time when earlier ones exist. */
        while (first < nev && ev[first].at + leap_shift(ev[first].at) < INT32_MIN) first++;
        count = first;
        while (count < nev && ev[count].at + leap_shift(ev[count].at) <= INT32_MAX) count++;
        count -= first;
        if (first > 0) pseudo = tidx[first - 1];
    }
    size_t nl = 0;
    for (size_t i = 0; i < nleaps; i++) {
        zt t = leaps[i].t + leap_shift(leaps[i].t) + (leaps[i].corr > 0 ? 0 : 0);
        if (!v1 || (t >= INT32_MIN && t <= INT32_MAX)) nl++;
    }
    /* Each indicator array is written only when one of its entries is set (an absent array means all 0). */
    size_t nstd = 0, nut = 0;
    for (size_t i = 0; fat && i < ntypes; i++) {
        if (types[i].isstd) nstd = ntypes;
        if (types[i].isut) nut = ntypes;
    }
    put(b, "TZif", 4);
    put(b, &version, 1);
    static const char zero[15];
    put(b, zero, 15);
    put32(b, (int64_t)nut);
    put32(b, (int64_t)nstd);
    put32(b, (int64_t)nl);
    put32(b, (int64_t)(count + (pseudo >= 0)));
    put32(b, (int64_t)ntypes);
    put32(b, (int64_t)nabbr);
    if (pseudo >= 0) put32(b, INT32_MIN);
    for (size_t i = first; i < first + count; i++) {
        zt t = ev[i].at + leap_shift(ev[i].at);
        if (wide) put64(b, t);
        else put32(b, t);
    }
    if (pseudo >= 0) {
        unsigned char c = (unsigned char)pseudo;
        put(b, &c, 1);
    }
    for (size_t i = first; i < first + count; i++) {
        unsigned char c = (unsigned char)tidx[i];
        put(b, &c, 1);
    }
    for (size_t i = 0; i < ntypes; i++) {
        put32(b, types[i].utoff);
        unsigned char c[2] = {(unsigned char)types[i].isdst, (unsigned char)abbr_at[i]};
        put(b, c, 2);
    }
    put(b, abbrs, nabbr);
    zt total = 0;
    for (size_t i = 0; i < nleaps; i++) {
        zt t = leaps[i].t + total;
        total += leaps[i].corr;
        if (v1 && (t < INT32_MIN || t > INT32_MAX)) continue;
        if (wide) put64(b, t);
        else put32(b, t);
        put32(b, total);
    }
    for (size_t i = 0; i < nstd; i++) {
        unsigned char c = (unsigned char)types[i].isstd;
        put(b, &c, 1);
    }
    for (size_t i = 0; i < nut; i++) {
        unsigned char c = (unsigned char)types[i].isut;
        put(b, &c, 1);
    }
}

static const char *outdir = "/usr/share/zoneinfo";

static char *out_path(const char *name) {
    size_t n = strlen(outdir) + strlen(name) + 2;
    char *p = xrealloc(NULL, n);
    snprintf(p, n, "%s/%s", outdir, name);
    return p;
}

static int make_parents(char *path) {
    for (char *p = path + 1; *p; p++) {
        if (*p != '/') continue;
        *p = 0;
        if (mkdir(path, 0755) != 0 && errno != EEXIST) {
            int e = errno;
            *p = '/';
            errno = e;
            return -1;
        }
        *p = '/';
    }
    return 0;
}

/* Files named by the source are made below the output directory only. The directory itself is the caller's
 * (its path is followed as given, and made when missing); below it each name is walked one component at a
 * time from a descriptor of it, making directories as needed, never following a symbolic link and never a
 * "." or ".." component. A link this run or an earlier one made, or one placed in the tree, therefore never
 * takes a write elsewhere (D-31.18). As the names are resolved from that descriptor, the path length limit
 * applies to the name, not to the output directory's path joined to it (observed). */
static int outfd = -1;

static int open_outdir(void) {
    if (outfd >= 0) return 0;
    char *p = out_path("");
    int rc = make_parents(p);
    free(p);
    if (rc != 0) return -1;
    outfd = open(outdir, O_RDONLY | O_DIRECTORY | O_CLOEXEC);
    return outfd < 0 ? -1 : 0;
}

/* The directory to hold NAME, made as needed, as a descriptor (the caller closes it when it is not outfd), with
 * *BASE its last component; -1 after the message, which names PATH (the name joined to the output directory)
 * or, for a directory, the name up to it after the source location FILE, LINE (observed wording). */
static int out_parent(const char *name, const char *path, const char *file, long line, const char **base) {
    if (open_outdir() != 0) {
        fprintf(stderr, "%s: Can't create directory %s: %s\n", progname, path, strerror(errno));
        return -1;
    }
    if (strlen(name) >= PATH_MAX) {
        fprintf(stderr, "%s: Can't remove %s: %s\n", progname, path, strerror(ENAMETOOLONG));
        return -1;
    }
    char comp[NAME_MAX + 1];
    int dfd = outfd;
    const char *p = name;
    for (const char *e; (e = strchr(p, '/')) != NULL; p = e + 1) {
        size_t n = (size_t)(e - p);
        int nfd = -1;
        if (n > NAME_MAX) errno = ENAMETOOLONG;
        else if (n == 0 || (n <= 2 && strspn(p, ".") >= n)) errno = EINVAL;
        else {
            memcpy(comp, p, n);
            comp[n] = 0;
            if (mkdirat(dfd, comp, 0755) == 0 || errno == EEXIST)
                nfd = openat(dfd, comp, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
        }
        int err = errno;
        if (dfd != outfd) close(dfd);
        if (nfd < 0) {
            fprintf(stderr, "\"%s\", line %ld: %s: Can't create directory %.*s: %s\n", file, line, progname,
                    (int)(e - name), name, strerror(err));
            return -1;
        }
        dfd = nfd;
    }
    if (strlen(p) > NAME_MAX || strcmp(p, ".") == 0 || strcmp(p, "..") == 0 || !*p) {
        fprintf(stderr, "%s: Can't create %s: %s\n", progname, path, strerror(strlen(p) > NAME_MAX ? ENAMETOOLONG : EINVAL));
        if (dfd != outfd) close(dfd);
        return -1;
    }
    *base = p;
    return dfd;
}

static int write_file(const char *name, const char *file, long line, const struct buf *b) {
    char *path = out_path(name);
    char tmp[64];
    const char *base;
    int rc = -1;
    int dfd = out_parent(name, path, file, line, &base);
    if (dfd < 0) goto out;
    /* A short private name beside the file, renamed over it once complete: any name fits its directory. */
    int fd = -1;
    for (unsigned i = 0; i < 100; i++) {
        snprintf(tmp, sizeof tmp, ".zic%ld-%u", (long)getpid(), i);
        fd = openat(dfd, tmp, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0644);
        if (fd >= 0 || errno != EEXIST) break;
    }
    if (fd < 0) {
        fprintf(stderr, "%s: Can't create %s: %s\n", progname, path, strerror(errno));
        goto out;
    }
    size_t off = 0;
    while (off < b->n) {
        ssize_t w = write(fd, b->p + off, b->n - off);
        if (w < 0) {
            if (errno == EINTR) continue;
            break;
        }
        off += (size_t)w;
    }
    if (off != b->n || close(fd) != 0) {
        fprintf(stderr, "%s: Error writing %s: %s\n", progname, path, strerror(errno));
        unlinkat(dfd, tmp, 0);
        goto out;
    }
    if (renameat(dfd, tmp, dfd, base) != 0) {
        fprintf(stderr, "%s: Can't create %s: %s\n", progname, path, strerror(errno));
        unlinkat(dfd, tmp, 0);
        goto out;
    }
    rc = 0;
out:
    if (dfd >= 0 && dfd != outfd) close(dfd);
    free(path);
    return rc;
}

static int write_zone(const struct zone *zn) {
    run_zone(zn);
    if (too_many) {
        error_at(zn->file, zn->line, "too many transitions in zone %s", zn->name);
        return -1;
    }
    tidy_events();
    char tz[1024];
    footer(zn, tz, sizeof tz);
    if (nleaps && have_expires) {
        /* With an expiring leap-second table the data ends at the expiry: no transitions after it, one to the
         * type then in force marks it, and no footer extends the zone past it. */
        size_t k = 0;
        while (k < nev && ev[k].at < expires_at) k++;
        struct ttype t = k ? ev[k - 1].t : initial_type;
        nev = k;
        push_event(expires_at, &t);
        tz[0] = 0;
        footer_version = 2;
    }
    const struct zline *lastline = &zlines[zn->first + zn->count - 1];
    if (!fat && *tz && lastline->rules_kind == RULES_NAMED && has_forever_rule(lastline)) {
        /* Slim output (observed on the reference): once every rule that lasts forever has taken effect in
         * the last line, the footer gives the rest; the transitions from the last of those first firings on
         * are left out. */
        size_t nf = 0;
        zt cut = INT64_MIN;
        for (size_t i = 0; i < lastline->nrules; i++)
            if (lastline->rules[i]->to == INT64_MAX) nf++;
        if (nf == nfiring && nf > 0) {
            for (size_t i = 0; i < nfiring; i++)
                if (firing_at[i] > cut) cut = firing_at[i];
            /* Never before a transition the footer cannot give: one made by a rule that ends. */
            if (last_finite_at != INT64_MIN && last_finite_at >= cut) cut = last_finite_at + 1;
            while (nev > 0 && ev[nev - 1].at >= cut) nev--;
        }
    }
    if (!fat && *tz && lastline->rules_kind == RULES_NAMED && has_forever_rule(lastline) && have_last_start &&
        (nev == 0 || ev[nev - 1].at < last_start)) {
        /* The footer describes the last line only: when nothing marks that line's start (its first type is
         * the one before it), a transition to that same type does, so a reader does not apply the footer to
         * the earlier line. */
        struct ttype t = nev ? ev[nev - 1].t : initial_type;
        push_event(last_start, &t);
    }
    if (fat && nev > 0 && strchr(tz, '<')) {
        /* Observed on the reference: a fat file whose footer quotes an abbreviation also marks the end of
         * the 32-bit range with a transition to the type then in force. */
        size_t at = 0;
        while (at < nev && ev[at].at < INT32_MAX) at++;
        if (at == nev || ev[at].at != INT32_MAX) {
            struct ttype t = at ? ev[at - 1].t : initial_type;
            push_event(INT32_MAX, &t);
            memmove(&ev[at + 1], &ev[at], (nev - 1 - at) * sizeof *ev);
            ev[at].at = INT32_MAX;
            ev[at].t = t;
        }
    }
    ntypes = 0;
    tidx = xrealloc(tidx, (nev + 1) * sizeof *tidx);
    struct ttype t0 = initial_type;
    type_of(&t0);
    for (size_t i = 0; i < nev; i++) tidx[i] = type_of(&ev[i].t);
    if (ntypes > 256) {
        error_at(zn->file, zn->line, "too many local time types in zone %s", zn->name);
        return -1;
    }
    /* Abbreviations, each once, in type order. */
    char abbrs[256 * 8 + 256];
    size_t nabbr = 0;
    int abbr_at[256];
    for (size_t i = 0; i < ntypes; i++) {
        size_t len = strlen(types[i].abbr);
        size_t j = 0;
        int found = -1;
        while (j < nabbr) {
            if (strcmp(abbrs + j, types[i].abbr) == 0) {
                found = (int)j;
                break;
            }
            j += strlen(abbrs + j) + 1;
        }
        if (found < 0) {
            if (nabbr + len + 1 > 255) {
                error_at(zn->file, zn->line, "too many or too long time zone abbreviations in zone %s", zn->name);
                return -1;
            }
            found = (int)nabbr;
            memcpy(abbrs + nabbr, types[i].abbr, len + 1);
            nabbr += len + 1;
        }
        abbr_at[i] = found;
    }
    char version = (char)('0' + footer_version);
    struct buf b = {0};
    if (fat) {
        write_block(&b, version, 1, abbrs, nabbr, abbr_at);
    } else {
        /* An empty version 1 block: one type of offset 0 and an empty abbreviation. */
        put(&b, "TZif", 4);
        put(&b, &version, 1);
        static const char zero[15];
        put(&b, zero, 15);
        for (int i = 0; i < 4; i++) put32(&b, 0);
        put32(&b, 1);
        put32(&b, 1);
        static const char t[7];
        put(&b, t, 7);
    }
    write_block(&b, version, 0, abbrs, nabbr, abbr_at);
    put(&b, "\n", 1);
    put(&b, tz, strlen(tz));
    put(&b, "\n", 1);
    int rc = write_file(zn->name, zn->file, zn->line, &b);
    free(b.p);
    return rc;
}

/* A link: a hard link to the target's file, or a relative symbolic link when that fails. */
static const char *resolve_link(const char *name, int depth) {
    if (depth > 64) return NULL;
    for (size_t i = 0; i < nzones; i++)
        if (strcmp(zones[i].name, name) == 0) return zones[i].name;
    for (size_t i = 0; i < nlinks; i++)
        if (strcmp(links[i].name, name) == 0) return resolve_link(links[i].target, depth + 1);
    return NULL;
}

/* SOURCE: a Link line (its name is walked below the output directory as a zone's is); else a link the caller
 * named by an option (-l, -p, -t), made at the path it gives. */
static int make_link(const char *target, const char *name, const char *file, long line, int source) {
    char *to = out_path(name);
    const char *real = resolve_link(target, 0);
    char *from = out_path(real ? real : target);
    char tmp[64];
    int have_tmp = 0;
    int rc = -1;
    int e = 0;
    int dfd = AT_FDCWD;
    const char *base = to;
    if (source) {
        /* The hard link is made at a private name at the top of the output directory first and renamed into
         * place, so a target that cannot be hard-linked is refused before any directory is made (observed). */
        if (open_outdir() == 0) {
            for (unsigned i = 0; i < 100; i++) {
                snprintf(tmp, sizeof tmp, ".zic%ld-l%u", (long)getpid(), i);
                if (linkat(AT_FDCWD, from, outfd, tmp, 0) == 0) {
                    have_tmp = 1;
                    break;
                }
                e = errno;
                if (e != EEXIST) break;
            }
        }
        if (!have_tmp && e == EPERM) goto refused;
        dfd = out_parent(name, to, file, line, &base);
        if (dfd < 0) goto out;
    } else if (make_parents(to) != 0) {
        fprintf(stderr, "%s: Can't create directory %s: %s\n", progname, to, strerror(errno));
        goto out;
    }
    if (unlinkat(dfd, base, 0) != 0 && errno != ENOENT) {
        fprintf(stderr, "%s: Can't remove %s: %s\n", progname, to, strerror(errno));
        goto out;
    }
    if (have_tmp) {
        if (renameat(outfd, tmp, dfd, base) == 0) {
            have_tmp = 0;
            rc = 0;
            goto out;
        }
        e = errno;
    } else if (!source) {
        if (linkat(AT_FDCWD, from, dfd, base, 0) == 0) {
            rc = 0;
            goto out;
        }
        e = errno;
        if (e == EPERM) goto refused;
    }
    {
        /* The path from the link's directory to the target; an absolute target is kept as given (observed). */
        size_t depth = 0;
        if (*target != '/')
            for (const char *p = name; *p; p++) depth += *p == '/';
        size_t n = depth * 3 + strlen(target) + 1;
        char *rel = xrealloc(NULL, n);
        rel[0] = 0;
        for (size_t i = 0; i < depth; i++) strcat(rel, "../");
        strcat(rel, target);
        if (symlinkat(rel, dfd, base) != 0) {
            fprintf(stderr, "%s: Can't link %s to %s: %s\n", progname, to, from, strerror(errno));
        } else {
            fprintf(stderr, "warning: \"%s\", line %ld: symbolic link used because hard link failed: %s\n", file,
                    line, strerror(e));
            rc = 0;
        }
        free(rel);
    }
    goto out;
refused:
    /* A target that cannot be hard-linked, such as a directory, is an error, not a symbolic link (observed). */
    fprintf(stderr, "%s: link from %s failed: %s\n", progname, from, strerror(e));
out:
    if (have_tmp) unlinkat(outfd, tmp, 0);
    if (dfd >= 0 && dfd != outfd) close(dfd);
    free(from);
    free(to);
    return rc;
}

static void resolve_rules(void) {
    for (size_t i = 0; i < nzlines; i++) {
        struct zline *z = &zlines[i];
        if (z->rules_kind != RULES_NAMED) continue;
        for (size_t j = 0; j < nrules; j++) {
            if (strcmp(rules[j].name, z->rules_name) != 0) continue;
            z->rules = xrealloc(z->rules, (z->nrules + 1) * sizeof *z->rules);
            z->rules[z->nrules++] = &rules[j];
        }
        if (z->nrules == 0) error_at(z->file, z->line, "invalid saved time");
    }
}

static void usage(FILE *f, int status) {
    fprintf(f,
            "%s: usage is %s [ --version ] [ --help ] [ -v ] \\\n"
            "\t[ -b {slim|fat} ] [ -d directory ] [ -l localtime ] [ -L leapseconds ] \\\n"
            "\t[ -p posixrules ] [ -r '[@lo][/@hi]' ] [ -t localtime-link ] \\\n"
            "\t[ filename ... ]\n",
            progname, progname);
    exit(status);
}

int main(int argc, char **argv) {
    const char *leapfile = NULL, *localtime_zone = NULL, *posixrules = NULL, *localtime_link = NULL;
    if (argc > 0 && argv[0]) progname = argv[0]; /* messages name the program as it was run */
    int opt;
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--version") == 0) {
            puts("zic (libniji) 2.39");
            return 0;
        }
        if (strcmp(argv[i], "--help") == 0) usage(stdout, 0);
        if (strcmp(argv[i], "--") == 0) break;
    }
    while ((opt = getopt(argc, argv, "b:d:l:L:p:r:st:vy:")) != -1) {
        switch (opt) {
        case 'b':
            if (strcmp(optarg, "slim") == 0) fat = 0;
            else if (strcmp(optarg, "fat") == 0) fat = 1;
            else {
                fprintf(stderr, "invalid option: -b '%s'\n", optarg);
                errors = 1;
            }
            break;
        case 'd': outdir = optarg; break;
        case 'l': localtime_zone = optarg; break;
        case 'L': leapfile = optarg; break;
        case 'p': posixrules = optarg; break;
        case 't': localtime_link = optarg; break;
        case 'r':
            fprintf(stderr, "%s: -r is not supported\n", progname);
            return 1;
        case 'v':
        case 's': break;
        case 'y': break;
        default: usage(stderr, 1);
        }
    }
    int bad_open = 0;
    if (leapfile && read_file(leapfile, 1)) bad_open = 1;
    if (optind == argc) {
        if (read_file("-", 0)) bad_open = 1;
    }
    for (int i = optind; i < argc; i++)
        if (read_file(argv[i], 0)) bad_open = 1;
    if (bad_open || errors) return 1;
    resolve_rules();
    for (size_t i = 0; i < nlinks; i++) {
        for (size_t j = 0; j < nzones; j++) {
            if (strcmp(links[i].name, zones[j].name) == 0) {
                error_at(links[i].file, links[i].line, "link %s targets itself or a zone of the same name",
                         links[i].name);
                break;
            }
        }
    }
    if (errors) return 1;
    int rc = 0;
    for (size_t i = 0; i < nzones; i++)
        if (write_zone(&zones[i])) rc = 1;
    for (size_t i = 0; i < nlinks; i++)
        if (make_link(links[i].target, links[i].name, links[i].file, links[i].line, 1)) rc = 1;
    if (localtime_zone && make_link(localtime_zone, localtime_link ? localtime_link : "localtime", "-l", 0, 0))
        rc = 1;
    if (posixrules && strcmp(posixrules, "-") != 0 && make_link(posixrules, "posixrules", "-p", 0, 0)) rc = 1;
    return rc || errors;
}
