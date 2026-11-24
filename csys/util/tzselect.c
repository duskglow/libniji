/* tzselect (UTIL-TZSELECT-PLDD, spec §31.11, plan/utilities.md): asks the user for a location on standard error and
 * standard input and prints the time-zone value for it on standard output, linked dynamically like zdump.
 *
 * The zone tables are the ones tzselect(1) names, read from $TZDIR (default /usr/share/zoneinfo): iso3166.tab (country
 * names) and zone1970.tab (zones, their countries, coordinates and comments); zonenow.tab is not needed (probed: the
 * reference runs without it and refuses to run without either of the other two).
 *
 * Every rule below that the man page leaves open was settled by black-box transcripts of the reference program
 * (handoff/UTIL-TZSELECT-PLDD.md lists them): the prompts and their texts, the menu layout (a numbered menu in
 * columns as wide as the longest entry plus two, filled column by column, padded with tabs then spaces, one entry
 * per line when every entry fits on one row; COLUMNS when it is a positive number, else 80), the answers a menu
 * takes (blanks around, an optional '+', leading zeros; an empty line shows the menu again), the order of the
 * continents and countries (the locale's collation of the names as shown), names in a locale whose codeset is not
 * UTF-8 (transliterated, '?' for what cannot be), the information block and its two times (the date(1) default
 * format of the POSIX locale), the TZ strings accepted, the labels of the -c list, and the exit codes (0 once
 * confirmed, 1 at the end of input or on a usage error). The -c list is in increasing great-circle distance
 * (D-31.34; differences row "tzselect -c order"). */
#define _GNU_SOURCE
#include <ctype.h>
#include <errno.h>
#include <iconv.h>
#include <langinfo.h>
#include <locale.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static const char *prog = "tzselect";
static int utf8 = 1;
static iconv_t conv = (iconv_t)-1;

struct zone {
    char *codes;   /* comma-separated country codes */
    char *coord;   /* ISO 6709 */
    char *tz;      /* the TZ value */
    char *comment; /* shown form; "" when none */
    double lat, lon;
};
struct country {
    char code[3];
    char *name; /* shown form */
};

static struct zone *zones;
static size_t nzones;
static struct country *countries;
static size_t ncountries;

static void *xrealloc(void *p, size_t n) {
    p = realloc(p, n ? n : 1);
    if (!p) {
        fprintf(stderr, "%s: out of memory\n", prog);
        exit(1);
    }
    return p;
}

static char *xstrdup(const char *s) {
    size_t n = strlen(s) + 1;
    return memcpy(xrealloc(0, n), s, n);
}

/* The shown form of a table string: unchanged in a UTF-8 locale, else converted to the locale's codeset with
 * transliteration, '?' for a character that has none. */
static char *shown(const char *s) {
    if (utf8 || conv == (iconv_t)-1) return xstrdup(s);
    size_t inl = strlen(s), cap = inl * 4 + 8, outl = cap - 1;
    char *out = xrealloc(0, cap), *o = out, *in = (char *)s;
    iconv(conv, 0, 0, 0, 0);
    while (inl > 0) {
        if (iconv(conv, &in, &inl, &o, &outl) != (size_t)-1) break;
        if (errno == E2BIG || outl == 0) break;
        /* EILSEQ or EINVAL: one '?' for the character, then on. */
        *o++ = '?';
        outl--;
        do {
            in++;
            inl--;
        } while (inl > 0 && ((unsigned char)*in & 0xc0) == 0x80);
    }
    *o = 0;
    return out;
}

/* Columns a shown string takes: characters in a UTF-8 locale, else bytes. */
static size_t width(const char *s) {
    size_t w = 0;
    for (; *s; s++)
        if (!utf8 || ((unsigned char)*s & 0xc0) != 0x80) w++;
    return w;
}

static char *slurp(const char *dir, const char *name) {
    size_t dl = strlen(dir), nl = strlen(name);
    char *path = xrealloc(0, dl + nl + 2);
    memcpy(path, dir, dl);
    path[dl] = '/';
    memcpy(path + dl + 1, name, nl + 1);
    FILE *f = fopen(path, "re");
    if (!f) {
        fprintf(stderr, "%s: %s: %s\n", prog, path, strerror(errno));
        fprintf(stderr, "%s: time zone files are not set up correctly\n", prog);
        exit(1);
    }
    size_t len = 0, cap = 4096;
    char *buf = xrealloc(0, cap);
    size_t got;
    while ((got = fread(buf + len, 1, cap - len - 1, f)) > 0) {
        len += got;
        if (cap - len < 2) buf = xrealloc(buf, cap *= 2);
    }
    fclose(f);
    buf[len] = 0;
    free(path);
    return buf;
}

/* One ISO 6709 component: sign, then `deg` digits of degrees, optionally minutes and seconds, optionally a
 * fraction of the last unit. Returns 0 on success. */
static int iso_part(const char **pp, int deg, double *out) {
    const char *p = *pp;
    if (*p != '+' && *p != '-') return -1;
    double sign = *p++ == '-' ? -1 : 1;
    const char *d = p;
    while (isdigit((unsigned char)*p)) p++;
    size_t n = (size_t)(p - d);
    if (n != (size_t)deg && n != (size_t)deg + 2 && n != (size_t)deg + 4) return -1;
    double v = 0, unit = 1;
    for (size_t i = 0; i < n;) {
        size_t take = i == 0 ? (size_t)deg : 2;
        double part = 0;
        for (size_t k = 0; k < take; k++) part = part * 10 + (d[i + k] - '0');
        v += part * unit;
        i += take;
        if (i < n) unit /= 60;
    }
    if (*p == '.') {
        p++;
        double scale = unit / 10;
        if (!isdigit((unsigned char)*p)) return -1;
        while (isdigit((unsigned char)*p)) {
            v += (*p++ - '0') * scale;
            scale /= 10;
        }
    }
    *pp = p;
    *out = sign * v;
    return 0;
}

/* Coordinates of a -c value or a table line; an unreadable value reads as 0, 0 (probed: the reference lists from
 * there). */
static void iso6709(const char *s, double *lat, double *lon) {
    const char *p = s;
    if (iso_part(&p, 2, lat) || iso_part(&p, 3, lon) || *p) *lat = *lon = 0;
}

/* sin x for |x| <= pi, by its Maclaurin series after folding x into [-pi/2, pi/2] (the last term kept, x^27/27!, is
 * below 1e-20 there). The program links libc only, not libm. */
static double sine(double x) {
    const double pi = 3.14159265358979323846;
    if (x > pi / 2) x = pi - x;
    if (x < -pi / 2) x = -pi - x;
    double term = x, sum = x;
    for (int k = 1; k <= 13; k++) {
        term *= -x * x / ((2.0 * k) * (2.0 * k + 1));
        sum += term;
    }
    return sum;
}

/* The haversine of the central angle between two points (degrees): sin^2(dlat/2) + cos lat1 cos lat2 sin^2(dlon/2).
 * It rises with the great-circle distance over [0, pi], so it orders the zones as the distance does. */
static double distance(double lat1, double lon1, double lat2, double lon2) {
    const double r = 3.14159265358979323846 / 180;
    double dlon = lon2 - lon1;
    while (dlon > 180) dlon -= 360;
    while (dlon < -180) dlon += 360;
    double a = sine((lat2 - lat1) * r / 2), b = sine(dlon * r / 2);
    double c1 = sine((90 - (lat1 < 0 ? -lat1 : lat1)) * r), c2 = sine((90 - (lat2 < 0 ? -lat2 : lat2)) * r);
    return a * a + c1 * c2 * b * b;
}

static void load(const char *dir) {
    char *iso = slurp(dir, "iso3166.tab"), *zt = slurp(dir, "zone1970.tab");
    for (char *line = strtok(iso, "\n"); line; line = strtok(0, "\n")) {
        if (*line == '#') continue;
        char *tab = strchr(line, '\t');
        if (!tab || tab - line != 2) continue;
        countries = xrealloc(countries, (ncountries + 1) * sizeof *countries);
        memcpy(countries[ncountries].code, line, 2);
        countries[ncountries].code[2] = 0;
        char *end = strchr(tab + 1, '\t');
        if (end) *end = 0;
        countries[ncountries++].name = shown(tab + 1);
    }
    for (char *line = strtok(zt, "\n"); line; line = strtok(0, "\n")) {
        if (*line == '#') continue;
        char *f[4] = {line, 0, 0, 0};
        for (int i = 1; i < 4; i++) {
            char *t = strchr(f[i - 1], '\t');
            if (!t) break;
            *t = 0;
            f[i] = t + 1;
        }
        if (!f[2] || !*f[2]) continue;
        zones = xrealloc(zones, (nzones + 1) * sizeof *zones);
        struct zone *z = &zones[nzones++];
        z->codes = f[0];
        z->coord = f[1];
        z->tz = f[2];
        z->comment = shown(f[3] ? f[3] : "");
        iso6709(z->coord, &z->lat, &z->lon);
    }
}

static const char *country_name(const char *code, size_t len) {
    for (size_t i = 0; i < ncountries; i++)
        if (strlen(countries[i].code) == len && !memcmp(countries[i].code, code, len)) return countries[i].name;
    return 0;
}

static int has_code(const struct zone *z, const char *code) {
    size_t n = strlen(code);
    for (const char *p = z->codes; *p;) {
        const char *e = strchr(p, ',');
        size_t l = e ? (size_t)(e - p) : strlen(p);
        if (l == n && !memcmp(p, code, n)) return 1;
        if (!e) break;
        p = e + 1;
    }
    return 0;
}

/* ---- input ---- */

static char *line_buf;
static size_t line_cap;

/* One answer line without its newline and surrounding blanks; a last line without a newline is the end of input,
 * as for the reference. The end of input ends the program with status 1, after a newline on standard output when it
 * came at a menu (probed). */
static char *answer(int at_menu) {
    ssize_t n = getline(&line_buf, &line_cap, stdin);
    if (n <= 0 || line_buf[n - 1] != '\n') {
        if (at_menu) putchar('\n');
        exit(1);
    }
    line_buf[--n] = 0;
    char *s = line_buf;
    while (*s == ' ' || *s == '\t') s++;
    while (n > 0 && (line_buf[n - 1] == ' ' || line_buf[n - 1] == '\t')) line_buf[--n] = 0;
    return s;
}

/* ---- menus ---- */

static int digits(size_t n) {
    int d = 1;
    while (n >= 10) {
        n /= 10;
        d++;
    }
    return d;
}

static size_t screen_columns(void) {
    const char *c = getenv("COLUMNS");
    if (c && *c) {
        char *end;
        errno = 0;
        long v = strtol(c, &end, 10);
        if (!*end && !errno && v > 0) return (size_t)v;
    }
    return 80;
}

static void show_menu(char **items, size_t n) {
    int dn = digits(n);
    size_t longest = 0;
    for (size_t i = 0; i < n; i++) {
        size_t w = (size_t)dn + 2 + width(items[i]);
        if (w > longest) longest = w;
    }
    size_t colw = longest + 2, cols = screen_columns() / colw;
    if (cols < 1) cols = 1;
    size_t rows = (n + cols - 1) / cols;
    if (rows == 1) rows = n; /* everything on one row: one entry per line instead */
    int dr = digits(rows);
    for (size_t r = 0; r < rows; r++) {
        size_t pos = 0;
        for (size_t c = 0, i = r; i < n; c++, i += rows) {
            size_t start = c * colw;
            while ((pos / 8 + 1) * 8 <= start) {
                fputc('\t', stderr);
                pos = (pos / 8 + 1) * 8;
            }
            for (; pos < start; pos++) fputc(' ', stderr);
            int nw = c == 0 ? dr : dn;
            fprintf(stderr, "%*zu) %s", nw, i + 1, items[i]);
            pos = start + (size_t)nw + 2 + width(items[i]);
        }
        fputc('\n', stderr);
    }
}

/* Shows the menu and returns the index chosen. */
static size_t menu(char **items, size_t n) {
    show_menu(items, n);
    for (;;) {
        fputs("#? ", stderr);
        char *a = answer(1);
        if (!*a) {
            show_menu(items, n);
            continue;
        }
        const char *p = a;
        if (*p == '+') p++;
        size_t v = 0;
        int ok = isdigit((unsigned char)*p);
        for (; ok && *p; p++) {
            if (!isdigit((unsigned char)*p)) ok = 0;
            else if (v <= n) v = v * 10 + (size_t)(*p - '0');
        }
        if (ok && v >= 1 && v <= n) return v - 1;
        fputs("Please enter a number in range.\n", stderr);
    }
}

/* ---- TZ strings (POSIX TZ, with the angle-bracket names) ---- */

static int tz_name(const char **pp) {
    const char *p = *pp;
    size_t n = 0;
    if (*p == '<') {
        for (p++; isalnum((unsigned char)*p) || *p == '+' || *p == '-'; p++) n++;
        if (*p++ != '>') return -1;
    } else {
        for (; isalpha((unsigned char)*p); p++) n++;
    }
    if (n < 3) return -1;
    *pp = p;
    return 0;
}

static int tz_num(const char **pp, int mind, int maxd, int max) {
    const char *p = *pp;
    int v = 0, n = 0;
    while (isdigit((unsigned char)*p) && n < maxd) {
        v = v * 10 + (*p++ - '0');
        n++;
    }
    if (n < mind || isdigit((unsigned char)*p) || v > max) return -1;
    *pp = p;
    return 0;
}

/* hh[:mm[:ss]], hours 0 to 24, an optional sign when `sign`. */
static int tz_time(const char **pp, int sign) {
    const char *p = *pp;
    if (sign && (*p == '+' || *p == '-')) p++;
    if (tz_num(&p, 1, 2, 24)) return -1;
    for (int i = 0; i < 2 && *p == ':'; i++) {
        p++;
        if (tz_num(&p, 2, 2, 59)) return -1;
    }
    *pp = p;
    return 0;
}

static int tz_rule(const char **pp) {
    const char *p = *pp;
    if (*p == 'J') {
        p++;
        if (tz_num(&p, 1, 3, 365) || !strtol(*pp + 1, 0, 10)) return -1; /* J1 to J365 */
    } else if (*p == 'M') {
        p++;
        if (tz_num(&p, 1, 2, 12) || !strtol(*pp + 1, 0, 10) || *p++ != '.') return -1; /* M1 to M12 */
        if (*p < '1' || *p > '5' || p[1] != '.') return -1;
        p += 2;
        if (*p < '0' || *p > '6') return -1;
        p++;
    } else if (tz_num(&p, 1, 3, 365)) {
        return -1;
    }
    if (*p == '/') {
        p++;
        if (tz_time(&p, 0)) return -1;
    }
    *pp = p;
    return 0;
}

static int tz_valid(const char *s) {
    if (*s == ':') return 1;
    const char *p = s;
    if (tz_name(&p) || tz_time(&p, 1)) return 0;
    if (!*p) return 1;
    if (tz_name(&p)) return 0;
    if (*p && *p != ',' && tz_time(&p, 1)) return 0;
    if (!*p) return 1;
    if (*p++ != ',' || tz_rule(&p) || *p++ != ',' || tz_rule(&p)) return 0;
    return !*p;
}

/* ---- the session ---- */

static const char *continent_name(const char *prefix, size_t n) {
    static const char *const names[][2] = {
        {"Africa", "Africa"},       {"America", "Americas"},         {"Antarctica", "Antarctica"},
        {"Arctic", "Arctic Ocean"}, {"Asia", "Asia"},                {"Atlantic", "Atlantic Ocean"},
        {"Australia", "Australia"}, {"Europe", "Europe"},            {"Indian", "Indian Ocean"},
        {"Pacific", "Pacific Ocean"},
    };
    for (size_t i = 0; i < sizeof names / sizeof names[0]; i++)
        if (strlen(names[i][0]) == n && !memcmp(names[i][0], prefix, n)) return names[i][1];
    return 0;
}

/* Menu order of names: the locale's collation; where that is byte order (C, POSIX, C.UTF-8), ASCII letters compare
 * as their capitals first (probed: "United States" before "US minor outlying islands"), then byte order. */
static int bytewise_collation;
static int by_coll(const void *a, const void *b) {
    const unsigned char *x = *(unsigned char *const *)a, *y = *(unsigned char *const *)b;
    if (!bytewise_collation) return strcoll((const char *)x, (const char *)y);
    for (size_t i = 0;; i++) {
        int cx = x[i] >= 'a' && x[i] <= 'z' ? x[i] - ('a' - 'A') : x[i];
        int cy = y[i] >= 'a' && y[i] <= 'z' ? y[i] - ('a' - 'A') : y[i];
        if (cx != cy) return cx - cy;
        if (!x[i]) return strcmp((const char *)x, (const char *)y);
    }
}

static const double *dist_of;
static int by_distance(const void *a, const void *b) {
    size_t x = *(const size_t *)a, y = *(const size_t *)b;
    if (dist_of[x] != dist_of[y]) return dist_of[x] < dist_of[y] ? -1 : 1;
    return x < y ? -1 : x > y; /* table order between equals */
}

/* The -c label of a zone: its countries' names, and its comment after " - ". The code US is shown as is (probed). */
static char *coord_label(const struct zone *z) {
    size_t cap = 64, len = 0;
    char *s = xrealloc(0, cap);
    s[0] = 0;
    for (const char *p = z->codes; *p;) {
        const char *e = strchr(p, ',');
        size_t l = e ? (size_t)(e - p) : strlen(p);
        const char *name = l == 2 && !memcmp(p, "US", 2) ? 0 : country_name(p, l);
        size_t nl = name ? strlen(name) : l;
        while (len + nl + 8 + strlen(z->comment) >= cap) s = xrealloc(s, cap *= 2);
        if (len) len += (size_t)sprintf(s + len, ", ");
        memcpy(s + len, name ? name : p, nl);
        s[len += nl] = 0;
        if (!e) break;
        p = e + 1;
    }
    if (*z->comment) sprintf(s + len, " - %s", z->comment);
    return s;
}

/* The -c list: the `limit` zones nearest `coord`; returns the zone chosen. */
static const struct zone *by_coordinates(const char *coord, size_t limit) {
    double lat, lon;
    iso6709(coord, &lat, &lon);
    double *d = xrealloc(0, nzones * sizeof *d);
    size_t *order = xrealloc(0, nzones * sizeof *order);
    for (size_t i = 0; i < nzones; i++) {
        d[i] = distance(lat, lon, zones[i].lat, zones[i].lon);
        order[i] = i;
    }
    dist_of = d;
    qsort(order, nzones, sizeof *order, by_distance);
    size_t n = nzones < limit ? nzones : limit;
    char **items = xrealloc(0, (n ? n : 1) * sizeof *items);
    for (size_t i = 0; i < n; i++) items[i] = coord_label(&zones[order[i]]);
    fprintf(stderr,
            "Please select one of the following timezones, echo listed roughly in increasing order of distance "
            "from %s.\n",
            coord);
    if (!n) exit(1);
    size_t k = menu(items, n);
    const struct zone *z = &zones[order[k]];
    fprintf(stderr, "\nThe following information has been given:\n\n\tcoord %s\n\t%s\n", coord, items[k]);
    for (size_t i = 0; i < n; i++) free(items[i]);
    free(items);
    free(order);
    free(d);
    return z;
}

static void show_time(const char *what, const char *tz, time_t now) {
    char buf[256];
    struct tm tm;
    setenv("TZ", tz, 1);
    tzset();
    if (!localtime_r(&now, &tm) || !strftime(buf, sizeof buf, "%a %b %e %H:%M:%S %Z %Y", &tm)) buf[0] = 0;
    fprintf(stderr, "%s\t%s.\n", what, buf);
}

static const char help_text[] =
    "Usage: tzselect [--version] [--help] [-c COORD] [-n LIMIT]\n"
    "Select a timezone interactively.\n"
    "\n"
    "Options:\n"
    "\n"
    "  -c COORD\n"
    "    Instead of asking for continent and then country and then city,\n"
    "    ask for selection from time zones whose largest cities\n"
    "    are closest to the location with geographical coordinates COORD.\n"
    "    COORD should use ISO 6709 notation, for example, '-c +4852+00220'\n"
    "    for Paris (in degrees and minutes, North and East), or\n"
    "    '-c -35-058' for Buenos Aires (in degrees, South and West).\n"
    "\n"
    "  -n LIMIT\n"
    "    Display at most LIMIT locations when -c is used (default 10).\n"
    "\n"
    "  --version\n"
    "    Output version information.\n"
    "\n"
    "  --help\n"
    "    Output this help.\n";

static void usage_error(const char *fmt, const char *arg) {
    fprintf(stderr, "%s: ", prog);
    fprintf(stderr, fmt, arg);
    fprintf(stderr, "\n%s: try '%s --help'\n", prog, prog);
    exit(1);
}

int main(int argc, char **argv) {
    if (argc > 0 && argv[0]) prog = argv[0]; /* messages name the program as it was run */
    const char *opt_coord = 0;
    size_t limit = 10;
    int i = 1;
    for (; i < argc; i++) {
        const char *a = argv[i];
        if (!strcmp(a, "--help")) {
            fputs(help_text, stdout);
            return 0;
        }
        if (!strcmp(a, "--version")) {
            puts("tzselect (libniji) 2.39");
            return 0;
        }
        if (!strcmp(a, "--")) {
            i++;
            break;
        }
        if (a[0] != '-' || !a[1]) break;
        for (const char *o = a + 1; *o; o++) {
            if (*o != 'c' && *o != 'n') {
                char opt[2] = {*o, 0};
                usage_error("illegal option -- %s", opt);
            }
            const char *v = o[1] ? o + 1 : (i + 1 < argc ? argv[++i] : 0);
            if (!v) usage_error("option requires an argument -- %s", *o == 'c' ? "c" : "n");
            if (*o == 'c') {
                opt_coord = v;
            } else {
                /* LIMIT: a positive decimal number (D-31.34). */
                char *end;
                errno = 0;
                unsigned long long l = strtoull(v, &end, 10);
                if (!isdigit((unsigned char)*v) || *end || errno || l == 0)
                    usage_error("-n %s: LIMIT must be a positive number", v);
                limit = l > (size_t)-1 ? (size_t)-1 : (size_t)l;
            }
            break;
        }
    }
    if (i < argc) { /* no "try" line here (probed) */
        fprintf(stderr, "%s: %s: unknown argument\n", prog, argv[i]);
        return 1;
    }

    setlocale(LC_CTYPE, "");
    setlocale(LC_COLLATE, "");
    bytewise_collation = strcoll("a", "B") > 0; /* byte order puts every capital first */
    const char *cs = nl_langinfo(CODESET);
    utf8 = cs && !strcmp(cs, "UTF-8");
    if (!utf8 && cs) {
        char to[128];
        snprintf(to, sizeof to, "%s//TRANSLIT", cs);
        conv = iconv_open(to, "UTF-8");
    }
    const char *dir = getenv("TZDIR");
    load(dir && *dir ? dir : "/usr/share/zoneinfo");

    for (;;) {
        fputs("Please identify a location so that time zone rules can be set correctly.\n", stderr);
        const char *tz = 0;
        char *chosen_tz = 0;
        if (opt_coord) {
            tz = by_coordinates(opt_coord, limit)->tz;
        } else {
            fputs("Please select a continent, ocean, \"coord\", or \"TZ\".\n", stderr);
            /* The continents and oceans that zone1970.tab names, in the collation order of their names. */
            char **items = 0;
            size_t n = 0;
            for (size_t z = 0; z < nzones; z++) {
                const char *slash = strchr(zones[z].tz, '/');
                if (!slash) continue;
                size_t pl = (size_t)(slash - zones[z].tz);
                const char *name = continent_name(zones[z].tz, pl);
                char *own = 0;
                if (!name) {
                    own = xrealloc(0, pl + 1);
                    memcpy(own, zones[z].tz, pl);
                    own[pl] = 0;
                    name = own;
                }
                size_t k = 0;
                while (k < n && strcmp(items[k], name)) k++;
                if (k == n) {
                    items = xrealloc(items, (n + 3) * sizeof *items);
                    items[n++] = own ? own : xstrdup(name);
                } else {
                    free(own);
                }
            }
            qsort(items, n, sizeof *items, by_coll);
            items = xrealloc(items, (n + 2) * sizeof *items);
            items[n] = xstrdup("coord - I want to use geographical coordinates.");
            items[n + 1] = xstrdup("TZ - I want to specify the timezone using the Posix TZ format.");
            size_t k = menu(items, n + 2);
            if (k == n) {
                fputs("Please enter coordinates in ISO 6709 notation.\n"
                      "For example, +4042-07403 stands for\n"
                      "40 degrees 42 minutes north, 74 degrees 3 minutes west.\n",
                      stderr);
                char *c = xstrdup(answer(0));
                tz = by_coordinates(c, limit)->tz;
                free(c);
            } else if (k == n + 1) {
                for (;;) {
                    fputs("Please enter the desired value of the TZ environment variable.\n"
                          "For example, AEST-10 is abbreviated AEST and is 10 hours\n"
                          "ahead (east) of Greenwich, with no daylight saving time.\n",
                          stderr);
                    char *v = answer(0);
                    if (tz_valid(v)) {
                        tz = chosen_tz = xstrdup(v);
                        break;
                    }
                    fprintf(stderr, "'%s' is not a conforming Posix timezone string.\n", v);
                }
                fprintf(stderr, "\nThe following information has been given:\n\n\tTZ='%s'\n", tz);
            } else {
                /* A continent: its countries, then the zones of the country chosen. */
                const char *cont = items[k];
                char **names = 0;
                size_t nn = 0;
                for (size_t z = 0; z < nzones; z++) {
                    const char *slash = strchr(zones[z].tz, '/');
                    if (!slash) continue;
                    size_t pl = (size_t)(slash - zones[z].tz);
                    const char *cn = continent_name(zones[z].tz, pl);
                    if (cn ? strcmp(cn, cont) : (strlen(cont) != pl || memcmp(cont, zones[z].tz, pl))) continue;
                    for (const char *p = zones[z].codes; *p;) {
                        const char *e = strchr(p, ',');
                        size_t l = e ? (size_t)(e - p) : strlen(p);
                        const char *name = country_name(p, l);
                        if (name) {
                            size_t j = 0;
                            while (j < nn && strcmp(names[j], name)) j++;
                            if (j == nn) {
                                names = xrealloc(names, (nn + 1) * sizeof *names);
                                names[nn++] = (char *)name;
                            }
                        }
                        if (!e) break;
                        p = e + 1;
                    }
                }
                if (!nn) exit(1);
                qsort(names, nn, sizeof *names, by_coll);
                size_t c = 0;
                if (nn > 1) {
                    fputs("Please select a country whose clocks agree with yours.\n", stderr);
                    c = menu(names, nn);
                }
                const char *code = 0;
                for (size_t j = 0; j < ncountries && !code; j++)
                    if (countries[j].name == names[c]) code = countries[j].code;
                size_t *zs = xrealloc(0, nzones * sizeof *zs), nz = 0;
                for (size_t z = 0; z < nzones; z++)
                    if (has_code(&zones[z], code)) zs[nz++] = z;
                size_t pick = 0;
                if (nz > 1) {
                    fputs("Please select one of the following timezones.\n", stderr);
                    char **labels = xrealloc(0, nz * sizeof *labels);
                    for (size_t j = 0; j < nz; j++) labels[j] = zones[zs[j]].comment;
                    pick = menu(labels, nz);
                    free(labels);
                }
                const struct zone *z = &zones[zs[pick]];
                tz = z->tz;
                fprintf(stderr, "\nThe following information has been given:\n\n\t%s\n", names[c]);
                if (*z->comment) fprintf(stderr, "\t%s\n", z->comment);
                free(zs);
                free(names);
            }
            for (size_t j = 0; j < n + 2; j++) free(items[j]);
            free(items);
        }
        fprintf(stderr, "\nTherefore TZ='%s' will be used.\n", tz);
        time_t now = time(0);
        /* Both clock lines are left out when the value's offset from UTC now has a seconds part (probed:
         * EST5:30:59, EST-24:59:59; a rule time with seconds keeps them). */
        struct tm probe;
        setenv("TZ", tz, 1);
        tzset();
        if (localtime_r(&now, &probe) && probe.tm_gmtoff % 60 == 0) {
            show_time("Selected time is now:", tz, now);
            show_time("Universal Time is now:", "UTC0", now);
        }
        fputs("Is the above information OK?\n", stderr);
        char *yn[] = {"Yes", "No"};
        if (menu(yn, 2) == 0) {
            printf("%s\n", tz);
            return fflush(stdout) ? 1 : 0;
        }
        free(chosen_tz);
    }
}
