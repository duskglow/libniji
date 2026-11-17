/* locale (UTIL-8, spec §31.7, plan/utilities.md §4): a thin front end over setlocale and nl_langinfo.
 *
 * Without operands it lists the locale environment; a keyword or category operand prints item values
 * (`-k` adds `keyword=`, `-c` the category name); `-a` lists the available locales (`-v` with their
 * LC_IDENTIFICATION fields), `-m` the charmaps. The keyword table below is the reference's keyword set
 * (black-box: `locale -k LC_<category>` for every category), each mapped to its public langinfo.h item.
 * Item kinds follow Appendix D.1.5: word items arrive as values, every other item as the address of its data. */
#define _GNU_SOURCE
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <langinfo.h>
#include <limits.h>
#include <locale.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>

#define LOCALE_DIR "/usr/lib/locale"
#define ARCHIVE LOCALE_DIR "/locale-archive"
#define CHARMAP_DIR "/usr/share/i18n/charmaps"

enum kind {
    STR,    /* a string */
    SARR,   /* `n` consecutive string items, one value joined by ';' */
    SLIST,  /* NUL-separated strings ended by an empty one, each its own value */
    BYTES,  /* bytes up to a NUL, as signed numbers joined by ';' (none: -1) */
    BYTE,   /* one signed byte */
    WORD,   /* a word item (the value itself) */
    WREF,   /* a word at the item's address */
    WARR2,  /* two words at the item's address */
};

struct kw {
    const char *name;
    int item;
    unsigned char kind, n;
};

#define K(name, item, kind) {name, item, kind, 1}
#define KA(name, item, n) {name, item, SARR, n}

static const struct kw ctype_kw[] = {
    K("ctype-class-names", _NL_CTYPE_CLASS_NAMES, SLIST), K("ctype-map-names", _NL_CTYPE_MAP_NAMES, SLIST),
    K("ctype-width", _NL_CTYPE_WIDTH, WREF), K("ctype-mb-cur-max", _NL_CTYPE_MB_CUR_MAX, WORD),
    K("charmap", CODESET, STR), K("ctype-class-offset", _NL_CTYPE_CLASS_OFFSET, WORD),
    K("ctype-map-offset", _NL_CTYPE_MAP_OFFSET, WORD), K("ctype-indigits_mb-len", _NL_CTYPE_INDIGITS_MB_LEN, WORD),
    K("ctype-indigits0_mb", _NL_CTYPE_INDIGITS0_MB, STR), K("ctype-indigits1_mb", _NL_CTYPE_INDIGITS1_MB, STR),
    K("ctype-indigits2_mb", _NL_CTYPE_INDIGITS2_MB, STR), K("ctype-indigits3_mb", _NL_CTYPE_INDIGITS3_MB, STR),
    K("ctype-indigits4_mb", _NL_CTYPE_INDIGITS4_MB, STR), K("ctype-indigits5_mb", _NL_CTYPE_INDIGITS5_MB, STR),
    K("ctype-indigits6_mb", _NL_CTYPE_INDIGITS6_MB, STR), K("ctype-indigits7_mb", _NL_CTYPE_INDIGITS7_MB, STR),
    K("ctype-indigits8_mb", _NL_CTYPE_INDIGITS8_MB, STR), K("ctype-indigits9_mb", _NL_CTYPE_INDIGITS9_MB, STR),
    K("ctype-indigits_wc-len", _NL_CTYPE_INDIGITS_WC_LEN, WORD),
    K("ctype-outdigit0_mb", _NL_CTYPE_OUTDIGIT0_MB, STR), K("ctype-outdigit1_mb", _NL_CTYPE_OUTDIGIT1_MB, STR),
    K("ctype-outdigit2_mb", _NL_CTYPE_OUTDIGIT2_MB, STR), K("ctype-outdigit3_mb", _NL_CTYPE_OUTDIGIT3_MB, STR),
    K("ctype-outdigit4_mb", _NL_CTYPE_OUTDIGIT4_MB, STR), K("ctype-outdigit5_mb", _NL_CTYPE_OUTDIGIT5_MB, STR),
    K("ctype-outdigit6_mb", _NL_CTYPE_OUTDIGIT6_MB, STR), K("ctype-outdigit7_mb", _NL_CTYPE_OUTDIGIT7_MB, STR),
    K("ctype-outdigit8_mb", _NL_CTYPE_OUTDIGIT8_MB, STR), K("ctype-outdigit9_mb", _NL_CTYPE_OUTDIGIT9_MB, STR),
    K("ctype-outdigit0_wc", _NL_CTYPE_OUTDIGIT0_WC, WORD), K("ctype-outdigit1_wc", _NL_CTYPE_OUTDIGIT1_WC, WORD),
    K("ctype-outdigit2_wc", _NL_CTYPE_OUTDIGIT2_WC, WORD), K("ctype-outdigit3_wc", _NL_CTYPE_OUTDIGIT3_WC, WORD),
    K("ctype-outdigit4_wc", _NL_CTYPE_OUTDIGIT4_WC, WORD), K("ctype-outdigit5_wc", _NL_CTYPE_OUTDIGIT5_WC, WORD),
    K("ctype-outdigit6_wc", _NL_CTYPE_OUTDIGIT6_WC, WORD), K("ctype-outdigit7_wc", _NL_CTYPE_OUTDIGIT7_WC, WORD),
    K("ctype-outdigit8_wc", _NL_CTYPE_OUTDIGIT8_WC, WORD), K("ctype-outdigit9_wc", _NL_CTYPE_OUTDIGIT9_WC, WORD),
    K("ctype-translit-tab-size", _NL_CTYPE_TRANSLIT_TAB_SIZE, WORD),
    K("ctype-translit-default-missing-len", _NL_CTYPE_TRANSLIT_DEFAULT_MISSING_LEN, WORD),
    K("ctype-translit-ignore-len", _NL_CTYPE_TRANSLIT_IGNORE_LEN, WORD),
    K("ctype-translit-ignore", _NL_CTYPE_TRANSLIT_IGNORE, STR),
    K("map-to-nonascii", _NL_CTYPE_MAP_TO_NONASCII, WORD), K("nonascii-case", _NL_CTYPE_NONASCII_CASE, WORD),
};
static const struct kw numeric_kw[] = {
    K("decimal_point", __DECIMAL_POINT, STR), K("thousands_sep", __THOUSANDS_SEP, STR),
    K("grouping", __GROUPING, BYTES), K("numeric-decimal-point-wc", _NL_NUMERIC_DECIMAL_POINT_WC, WORD),
    K("numeric-thousands-sep-wc", _NL_NUMERIC_THOUSANDS_SEP_WC, WORD), K("numeric-codeset", _NL_NUMERIC_CODESET, STR),
};
static const struct kw time_kw[] = {
    KA("abday", ABDAY_1, 7), KA("day", DAY_1, 7), KA("abmon", ABMON_1, 12), KA("mon", MON_1, 12),
    KA("am_pm", AM_STR, 2), K("d_t_fmt", D_T_FMT, STR), K("d_fmt", D_FMT, STR), K("t_fmt", T_FMT, STR),
    K("t_fmt_ampm", T_FMT_AMPM, STR), K("era", ERA, SLIST), K("era_year", __ERA_YEAR, STR),
    K("era_d_fmt", ERA_D_FMT, STR), K("alt_digits", ALT_DIGITS, SLIST), K("era_d_t_fmt", ERA_D_T_FMT, STR),
    K("era_t_fmt", ERA_T_FMT, STR), K("time-era-num-entries", _NL_TIME_ERA_NUM_ENTRIES, WORD),
    K("time-era-entries", _NL_TIME_ERA_ENTRIES, STR), K("week-ndays", _NL_TIME_WEEK_NDAYS, BYTE),
    K("week-1stday", _NL_TIME_WEEK_1STDAY, WORD), K("week-1stweek", _NL_TIME_WEEK_1STWEEK, BYTE),
    K("first_weekday", _NL_TIME_FIRST_WEEKDAY, BYTE), K("first_workday", _NL_TIME_FIRST_WORKDAY, BYTE),
    K("cal_direction", _NL_TIME_CAL_DIRECTION, BYTE), K("timezone", _NL_TIME_TIMEZONE, STR),
    K("date_fmt", _DATE_FMT, STR), K("time-codeset", _NL_TIME_CODESET, STR), KA("alt_mon", ALTMON_1, 12),
    KA("ab_alt_mon", _NL_ABALTMON_1, 12),
};
static const struct kw collate_kw[] = {
    K("collate-nrules", _NL_COLLATE_NRULES, WORD), K("collate-rulesets", _NL_COLLATE_RULESETS, STR),
    K("collate-symb-hash-sizemb", _NL_COLLATE_SYMB_HASH_SIZEMB, WORD), K("collate-codeset", _NL_COLLATE_CODESET, STR),
};
static const struct kw monetary_kw[] = {
    K("int_curr_symbol", __INT_CURR_SYMBOL, STR), K("currency_symbol", __CURRENCY_SYMBOL, STR),
    K("mon_decimal_point", __MON_DECIMAL_POINT, STR), K("mon_thousands_sep", __MON_THOUSANDS_SEP, STR),
    K("mon_grouping", __MON_GROUPING, BYTES), K("positive_sign", __POSITIVE_SIGN, STR),
    K("negative_sign", __NEGATIVE_SIGN, STR), K("int_frac_digits", __INT_FRAC_DIGITS, BYTE),
    K("frac_digits", __FRAC_DIGITS, BYTE), K("p_cs_precedes", __P_CS_PRECEDES, BYTE),
    K("p_sep_by_space", __P_SEP_BY_SPACE, BYTE), K("n_cs_precedes", __N_CS_PRECEDES, BYTE),
    K("n_sep_by_space", __N_SEP_BY_SPACE, BYTE), K("p_sign_posn", __P_SIGN_POSN, BYTE),
    K("n_sign_posn", __N_SIGN_POSN, BYTE), K("crncystr", _NL_MONETARY_CRNCYSTR, STR),
    K("int_p_cs_precedes", __INT_P_CS_PRECEDES, BYTE), K("int_p_sep_by_space", __INT_P_SEP_BY_SPACE, BYTE),
    K("int_n_cs_precedes", __INT_N_CS_PRECEDES, BYTE), K("int_n_sep_by_space", __INT_N_SEP_BY_SPACE, BYTE),
    K("int_p_sign_posn", __INT_P_SIGN_POSN, BYTE), K("int_n_sign_posn", __INT_N_SIGN_POSN, BYTE),
    K("duo_int_curr_symbol", _NL_MONETARY_DUO_INT_CURR_SYMBOL, STR),
    K("duo_currency_symbol", _NL_MONETARY_DUO_CURRENCY_SYMBOL, STR),
    K("duo_int_frac_digits", _NL_MONETARY_DUO_INT_FRAC_DIGITS, BYTE),
    K("duo_frac_digits", _NL_MONETARY_DUO_FRAC_DIGITS, BYTE),
    K("duo_p_cs_precedes", _NL_MONETARY_DUO_P_CS_PRECEDES, BYTE),
    K("duo_p_sep_by_space", _NL_MONETARY_DUO_P_SEP_BY_SPACE, BYTE),
    K("duo_n_cs_precedes", _NL_MONETARY_DUO_N_CS_PRECEDES, BYTE),
    K("duo_n_sep_by_space", _NL_MONETARY_DUO_N_SEP_BY_SPACE, BYTE),
    K("duo_int_p_cs_precedes", _NL_MONETARY_DUO_INT_P_CS_PRECEDES, BYTE),
    K("duo_int_p_sep_by_space", _NL_MONETARY_DUO_INT_P_SEP_BY_SPACE, BYTE),
    K("duo_int_n_cs_precedes", _NL_MONETARY_DUO_INT_N_CS_PRECEDES, BYTE),
    K("duo_int_n_sep_by_space", _NL_MONETARY_DUO_INT_N_SEP_BY_SPACE, BYTE),
    K("duo_p_sign_posn", _NL_MONETARY_DUO_P_SIGN_POSN, BYTE),
    K("duo_n_sign_posn", _NL_MONETARY_DUO_N_SIGN_POSN, BYTE),
    K("duo_int_p_sign_posn", _NL_MONETARY_DUO_INT_P_SIGN_POSN, BYTE),
    K("duo_int_n_sign_posn", _NL_MONETARY_DUO_INT_N_SIGN_POSN, BYTE),
    K("uno_valid_from", _NL_MONETARY_UNO_VALID_FROM, WORD), K("uno_valid_to", _NL_MONETARY_UNO_VALID_TO, WORD),
    K("duo_valid_from", _NL_MONETARY_DUO_VALID_FROM, WORD), K("duo_valid_to", _NL_MONETARY_DUO_VALID_TO, WORD),
    K("conversion_rate", _NL_MONETARY_CONVERSION_RATE, WARR2),
    K("monetary-decimal-point-wc", _NL_MONETARY_DECIMAL_POINT_WC, WORD),
    K("monetary-thousands-sep-wc", _NL_MONETARY_THOUSANDS_SEP_WC, WORD),
    K("monetary-codeset", _NL_MONETARY_CODESET, STR),
};
static const struct kw messages_kw[] = {
    K("yesexpr", __YESEXPR, STR), K("noexpr", __NOEXPR, STR), K("yesstr", __YESSTR, STR),
    K("nostr", __NOSTR, STR), K("messages-codeset", _NL_MESSAGES_CODESET, STR),
};
static const struct kw paper_kw[] = {
    K("height", _NL_PAPER_HEIGHT, WORD), K("width", _NL_PAPER_WIDTH, WORD),
    K("paper-codeset", _NL_PAPER_CODESET, STR),
};
static const struct kw name_kw[] = {
    K("name_fmt", _NL_NAME_NAME_FMT, STR), K("name_gen", _NL_NAME_NAME_GEN, STR),
    K("name_mr", _NL_NAME_NAME_MR, STR), K("name_mrs", _NL_NAME_NAME_MRS, STR),
    K("name_miss", _NL_NAME_NAME_MISS, STR), K("name_ms", _NL_NAME_NAME_MS, STR),
    K("name-codeset", _NL_NAME_CODESET, STR),
};
static const struct kw address_kw[] = {
    K("postal_fmt", _NL_ADDRESS_POSTAL_FMT, STR), K("country_name", _NL_ADDRESS_COUNTRY_NAME, STR),
    K("country_post", _NL_ADDRESS_COUNTRY_POST, STR), K("country_ab2", _NL_ADDRESS_COUNTRY_AB2, STR),
    K("country_ab3", _NL_ADDRESS_COUNTRY_AB3, STR), K("country_car", _NL_ADDRESS_COUNTRY_CAR, STR),
    K("country_num", _NL_ADDRESS_COUNTRY_NUM, WORD), K("country_isbn", _NL_ADDRESS_COUNTRY_ISBN, STR),
    K("lang_name", _NL_ADDRESS_LANG_NAME, STR), K("lang_ab", _NL_ADDRESS_LANG_AB, STR),
    K("lang_term", _NL_ADDRESS_LANG_TERM, STR), K("lang_lib", _NL_ADDRESS_LANG_LIB, STR),
    K("address-codeset", _NL_ADDRESS_CODESET, STR),
};
static const struct kw telephone_kw[] = {
    K("tel_int_fmt", _NL_TELEPHONE_TEL_INT_FMT, STR), K("tel_dom_fmt", _NL_TELEPHONE_TEL_DOM_FMT, STR),
    K("int_select", _NL_TELEPHONE_INT_SELECT, STR), K("int_prefix", _NL_TELEPHONE_INT_PREFIX, STR),
    K("telephone-codeset", _NL_TELEPHONE_CODESET, STR),
};
static const struct kw measurement_kw[] = {
    K("measurement", _NL_MEASUREMENT_MEASUREMENT, BYTE), K("measurement-codeset", _NL_MEASUREMENT_CODESET, STR),
};
static const struct kw identification_kw[] = {
    K("title", _NL_IDENTIFICATION_TITLE, STR), K("source", _NL_IDENTIFICATION_SOURCE, STR),
    K("address", _NL_IDENTIFICATION_ADDRESS, STR), K("contact", _NL_IDENTIFICATION_CONTACT, STR),
    K("email", _NL_IDENTIFICATION_EMAIL, STR), K("tel", _NL_IDENTIFICATION_TEL, STR),
    K("fax", _NL_IDENTIFICATION_FAX, STR), K("language", _NL_IDENTIFICATION_LANGUAGE, STR),
    K("territory", _NL_IDENTIFICATION_TERRITORY, STR), K("audience", _NL_IDENTIFICATION_AUDIENCE, STR),
    K("application", _NL_IDENTIFICATION_APPLICATION, STR), K("abbreviation", _NL_IDENTIFICATION_ABBREVIATION, STR),
    K("revision", _NL_IDENTIFICATION_REVISION, STR), K("date", _NL_IDENTIFICATION_DATE, STR),
    K("category", _NL_IDENTIFICATION_CATEGORY, STR), K("identification-codeset", _NL_IDENTIFICATION_CODESET, STR),
};

#define CAT(name, tab) {name, tab, sizeof tab / sizeof tab[0]}
static const struct cat {
    const char *name;
    const struct kw *kw;
    size_t n;
} cats[] = {
    CAT("LC_CTYPE", ctype_kw), CAT("LC_NUMERIC", numeric_kw), CAT("LC_TIME", time_kw),
    CAT("LC_COLLATE", collate_kw), CAT("LC_MONETARY", monetary_kw), CAT("LC_MESSAGES", messages_kw),
    CAT("LC_PAPER", paper_kw), CAT("LC_NAME", name_kw), CAT("LC_ADDRESS", address_kw),
    CAT("LC_TELEPHONE", telephone_kw), CAT("LC_MEASUREMENT", measurement_kw),
    CAT("LC_IDENTIFICATION", identification_kw),
};
#define NCATS (sizeof cats / sizeof cats[0])

static int opt_k, opt_c, opt_v;

static void quoted(const char *s, int q) {
    if (q) putchar('"');
    fputs(s, stdout);
    if (q) putchar('"');
}

static void value(const struct kw *k) {
    const char *p = nl_langinfo(k->item);
    if (opt_k) printf("%s=", k->name);
    switch (k->kind) {
    case STR:
        quoted(p, opt_k);
        break;
    case SARR:
        if (opt_k) putchar('"');
        for (int i = 0; i < k->n; i++) printf("%s%s", i ? ";" : "", nl_langinfo(k->item + i));
        if (opt_k) putchar('"');
        break;
    case SLIST:
        for (int first = 1; *p; p += strlen(p) + 1, first = 0) {
            if (!first) putchar(';');
            quoted(p, opt_k);
        }
        break;
    case BYTES:
        if (!*p) fputs("-1", stdout);
        for (const char *b = p; *b; b++) printf("%s%d", b == p ? "" : ";", (signed char)*b);
        break;
    case BYTE:
        printf("%d", (signed char)*p);
        break;
    case WORD:
        printf("%u", (unsigned)(uintptr_t)p);
        break;
    case WREF: {
        uint32_t w;
        memcpy(&w, p, sizeof w);
        printf("%u", w);
        break;
    }
    case WARR2: {
        int32_t w[2];
        memcpy(w, p, sizeof w);
        printf("%d;%d", w[0], w[1]);
        break;
    }
    }
    putchar('\n');
}

/* One operand: a category prints all its keywords, a keyword its value. 0 if the name is unknown. */
static int show(const char *name) {
    for (size_t c = 0; c < NCATS; c++) {
        if (strcmp(name, cats[c].name) == 0) {
            if (opt_c) puts(cats[c].name);
            for (size_t i = 0; i < cats[c].n; i++) value(&cats[c].kw[i]);
            return 1;
        }
        for (size_t i = 0; i < cats[c].n; i++)
            if (strcmp(name, cats[c].kw[i].name) == 0) {
                if (opt_c) puts(cats[c].name);
                value(&cats[c].kw[i]);
                return 1;
            }
    }
    return 0;
}

/* POSIX XBD 8.2: LC_ALL overrides, then the category's own variable, then LANG, then the default. A value that
 * comes from the variable itself is printed bare, an implied one quoted. */
static void show_environment(void) {
    const char *lang = getenv("LANG"), *all = getenv("LC_ALL"), *language = getenv("LANGUAGE");
    printf("LANG=%s\nLANGUAGE=%s\n", lang ? lang : "", language ? language : "");
    for (size_t c = 0; c < NCATS; c++) {
        const char *own = getenv(cats[c].name);
        if (all && *all)
            printf("%s=\"%s\"\n", cats[c].name, all);
        else if (own)
            printf("%s=%s\n", cats[c].name, own);
        else
            printf("%s=\"%s\"\n", cats[c].name, lang && *lang ? lang : "POSIX");
    }
    printf("LC_ALL=%s\n", all ? all : "");
}

/* A listed name and where `-v` says it comes from (NULL: built in, or a charmap). One array of pairs, so that
   sorting keeps each name with its own source. */
struct entry {
    char *name;
    const char *where;
};

struct names {
    struct entry *v;
    size_t n, cap;
};

static int by_collation(const void *a, const void *b) {
    return strcoll(((const struct entry *)a)->name, ((const struct entry *)b)->name);
}

static void add(struct names *s, const char *name, const char *where) {
    for (size_t i = 0; i < s->n; i++)
        if (strcmp(s->v[i].name, name) == 0) return;
    if (s->n == s->cap) {
        size_t cap = s->cap ? 2 * s->cap : 64;
        struct entry *v = realloc(s->v, cap * sizeof *v);
        if (!v) {
            perror("locale");
            exit(4);
        }
        s->v = v, s->cap = cap;
    }
    char *copy = strdup(name);
    if (!copy) {
        perror("locale");
        exit(4);
    }
    s->v[s->n++] = (struct entry){copy, where};
}

/* The archive's names (Appendix D.1.6): the name table is scanned, every offset checked against the file. */
static void archive_names(struct names *s) {
    int fd = open(ARCHIVE, O_RDONLY | O_CLOEXEC);
    struct stat st;
    if (fd < 0) return;
    if (fstat(fd, &st) != 0 || st.st_size < 20) {
        close(fd);
        return;
    }
    size_t size = (size_t)st.st_size;
    const unsigned char *m = mmap(NULL, size, PROT_READ, MAP_PRIVATE, fd, 0);
    close(fd);
    if (m == MAP_FAILED) return;
    uint32_t h[5];
    memcpy(h, m, sizeof h);
    if (h[0] == 0xde020109u && h[2] <= size && h[4] <= (size - h[2]) / 12) {
        for (uint32_t i = 0; i < h[4]; i++) {
            uint32_t e[3];
            memcpy(e, m + h[2] + 12u * i, sizeof e);
            if (e[1] == 0 || e[1] >= size || !memchr(m + e[1], 0, size - e[1])) continue;
            add(s, (const char *)m + e[1], "archive: " ARCHIVE);
        }
    }
    munmap((void *)m, size);
}

static void list_locales(void) {
    struct names s = {0};
    archive_names(&s);
    DIR *d = opendir(LOCALE_DIR);
    if (d) {
        struct dirent *e;
        while ((e = readdir(d))) {
            char path[PATH_MAX];
            struct stat st;
            /* Every directory holding LC_IDENTIFICATION, hidden ones too; only `.` and `..` are skipped. */
            if (strcmp(e->d_name, ".") == 0 || strcmp(e->d_name, "..") == 0) continue;
            snprintf(path, sizeof path, LOCALE_DIR "/%s/LC_IDENTIFICATION", e->d_name);
            if (stat(path, &st) == 0 && S_ISREG(st.st_mode)) add(&s, e->d_name, "directory");
        }
        closedir(d);
    }
    /* Built in, listed always; added last so that an installed C.utf8 keeps its directory for `-v`. */
    add(&s, "C", NULL);
    add(&s, "C.utf8", NULL);
    add(&s, "POSIX", NULL);
    qsort(s.v, s.n, sizeof *s.v, by_collation);
    for (size_t i = 0, shown = 0; i < s.n; i++) {
        const struct entry *e = &s.v[i];
        if (!opt_v) {
            puts(e->name);
            continue;
        }
        if (!e->where) continue;
        if (shown++) putchar('\n');
        if (strcmp(e->where, "directory") == 0)
            printf("locale: %-15.15s directory: " LOCALE_DIR "/%s\n", e->name, e->name);
        else
            printf("locale: %-15.15s %s\n", e->name, e->where);
        puts("-------------------------------------------------------------------------------");
        locale_t l = newlocale(LC_IDENTIFICATION_MASK | LC_CTYPE_MASK, e->name, (locale_t)0);
        if (!l) continue;
        static const struct {
            const char *label;
            int item;
        } f[] = {
            {"title", _NL_IDENTIFICATION_TITLE},       {"source", _NL_IDENTIFICATION_SOURCE},
            {"address", _NL_IDENTIFICATION_ADDRESS},   {"contact", _NL_IDENTIFICATION_CONTACT},
            {"email", _NL_IDENTIFICATION_EMAIL},       {"tel", _NL_IDENTIFICATION_TEL},
            {"fax", _NL_IDENTIFICATION_FAX},           {"language", _NL_IDENTIFICATION_LANGUAGE},
            {"territory", _NL_IDENTIFICATION_TERRITORY}, {"audience", _NL_IDENTIFICATION_AUDIENCE},
            {"application", _NL_IDENTIFICATION_APPLICATION}, {"abbreviation", _NL_IDENTIFICATION_ABBREVIATION},
            {"revision", _NL_IDENTIFICATION_REVISION}, {"date", _NL_IDENTIFICATION_DATE},
            {"codeset", CODESET},
        };
        for (size_t j = 0; j < sizeof f / sizeof f[0]; j++) {
            const char *v = nl_langinfo_l(f[j].item, l);
            if (v && *v) printf("%9s | %s\n", f[j].label, v);
        }
        freelocale(l);
    }
}

/* The start of a gzip-compressed charmap (RFC 1952 around RFC 1951 deflate), in process (ruling #6246; the same
   method as core::charmap::inflate's gunzip_prefix, D-6.8): output stops at the buffer's end, the input's end or
   the first malformed bit, and what was produced until then is kept. Only the header lines are read from it. */
#define HEADER_MAX 65536

struct bits {
    const unsigned char *p;
    size_t n, pos;
    unsigned bit;
};

static int bit(struct bits *b) {
    if (b->pos >= b->n) return -1;
    int v = (b->p[b->pos] >> b->bit) & 1;
    if (++b->bit == 8) b->bit = 0, b->pos++;
    return v;
}

static long bits(struct bits *b, unsigned n) {
    long v = 0;
    for (unsigned i = 0; i < n; i++) {
        int x = bit(b);
        if (x < 0) return -1;
        v |= (long)x << i;
    }
    return v;
}

/* A canonical Huffman code: the number of codes of each length, and the symbols in code order. */
struct huffman {
    unsigned short count[16], sym[320];
};

static int huffman(struct huffman *h, const unsigned char *len, unsigned n) {
    unsigned short offs[16];
    memset(h->count, 0, sizeof h->count);
    for (unsigned i = 0; i < n; i++) {
        if (len[i] > 15) return -1;
        h->count[len[i]]++;
    }
    h->count[0] = 0;
    offs[1] = 0;
    for (unsigned l = 1; l < 15; l++) offs[l + 1] = offs[l] + h->count[l];
    for (unsigned i = 0; i < n; i++)
        if (len[i]) h->sym[offs[len[i]]++] = (unsigned short)i;
    return 0;
}

static int decode(const struct huffman *h, struct bits *b) {
    int code = 0, first = 0, index = 0;
    for (int len = 1; len < 16; len++) {
        int x = bit(b);
        if (x < 0) return -1;
        code |= x;
        int count = h->count[len];
        if (code - count < first) return h->sym[index + (code - first)];
        index += count;
        first = (first + count) << 1;
        code <<= 1;
    }
    return -1;
}

static const unsigned short len_base[29] = {3,  4,  5,  6,  7,  8,  9,  10, 11,  13,  15,  17,  19,  23, 27,
                                            31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258};
static const unsigned char len_extra[29] = {0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2,
                                            2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0};
static const unsigned short dist_base[30] = {1,   2,   3,   4,   5,   7,    9,    13,   17,   25,
                                             33,  49,  65,  97,  129, 193,  257,  385,  513,  769,
                                             1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577};
static const unsigned char dist_extra[30] = {0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6,
                                             6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13};

/* One compressed block into out[*n..HEADER_MAX): 0 at its end-of-block code, -1 when it stops early. */
static int block(struct bits *b, const struct huffman *lit, const struct huffman *dist, unsigned char *out,
                 size_t *n) {
    for (;;) {
        int sym = decode(lit, b);
        if (sym < 0) return -1;
        if (sym < 256) {
            if (*n >= HEADER_MAX) return -1;
            out[(*n)++] = (unsigned char)sym;
            continue;
        }
        if (sym == 256) return 0;
        sym -= 257;
        if (sym >= 29) return -1;
        long extra = bits(b, len_extra[sym]);
        if (extra < 0) return -1;
        size_t len = len_base[sym] + (size_t)extra;
        int d = decode(dist, b);
        if (d < 0 || d >= 30) return -1;
        long dextra = bits(b, dist_extra[d]);
        if (dextra < 0) return -1;
        size_t back = dist_base[d] + (size_t)dextra;
        if (back > *n) return -1;
        for (size_t k = 0; k < len; k++) {
            if (*n >= HEADER_MAX) return -1;
            out[*n] = out[*n - back];
            (*n)++;
        }
    }
}

static int dynamic(struct bits *b, struct huffman *lit, struct huffman *dist) {
    static const unsigned char order[19] = {16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15};
    long hlit = bits(b, 5), hdist = bits(b, 5), hclen = bits(b, 4);
    if (hlit < 0 || hdist < 0 || hclen < 0) return -1;
    hlit += 257, hdist += 1, hclen += 4;
    unsigned char cl[19] = {0}, lengths[320] = {0};
    for (long i = 0; i < hclen; i++) {
        long v = bits(b, 3);
        if (v < 0) return -1;
        cl[order[i]] = (unsigned char)v;
    }
    struct huffman clh;
    if (huffman(&clh, cl, 19) < 0) return -1;
    long n = 0;
    while (n < hlit + hdist) {
        int sym = decode(&clh, b);
        long value = 0, repeat = 1;
        if (sym < 0) return -1;
        if (sym < 16)
            value = sym;
        else {
            /* 16: the previous length 3-6 times; 17: zero 3-10 times; 18: zero 11-138 times. */
            long r = bits(b, sym == 16 ? 2 : sym == 17 ? 3 : 7);
            if (r < 0 || (sym == 16 && n == 0)) return -1;
            repeat = r + (sym == 18 ? 11 : 3);
            if (sym == 16) value = lengths[n - 1];
        }
        if (n + repeat > hlit + hdist) return -1;
        while (repeat--) lengths[n++] = (unsigned char)value;
    }
    if (huffman(lit, lengths, (unsigned)hlit) < 0) return -1;
    return huffman(dist, lengths + hlit, (unsigned)hdist);
}

/* The text at the start of a gzip file `data`, into out (HEADER_MAX bytes); returns its length. */
static size_t gunzip_prefix(const unsigned char *data, size_t size, unsigned char *out) {
    if (size < 10 || data[0] != 0x1f || data[1] != 0x8b || data[2] != 8) return 0;
    unsigned flags = data[3];
    size_t p = 10, n = 0;
    if (flags & 4) {
        if (size - p < 2) return 0;
        p += 2 + (size_t)(data[p] | data[p + 1] << 8);
    }
    for (unsigned flag = 8; flag <= 16; flag <<= 1) {
        if (!(flags & flag)) continue;
        const unsigned char *z = p < size ? memchr(data + p, 0, size - p) : NULL;
        if (!z) return 0;
        p = (size_t)(z - data) + 1;
    }
    if (flags & 2) p += 2;
    if (p >= size) return 0;
    struct bits b = {data + p, size - p, 0, 0};
    for (;;) {
        int last = bit(&b);
        long type = bits(&b, 2);
        if (last < 0 || type < 0) return n;
        if (type == 0) {
            if (b.bit) b.bit = 0, b.pos++;
            if (b.n - b.pos < 4) return n;
            const unsigned char *q = b.p + b.pos;
            unsigned len = q[0] | q[1] << 8, nlen = q[2] | q[3] << 8;
            if (len != (~nlen & 0xffffu)) return n;
            b.pos += 4;
            for (unsigned k = 0; k < len; k++) {
                if (b.pos >= b.n || n >= HEADER_MAX) return n;
                out[n++] = b.p[b.pos++];
            }
        } else if (type == 1) {
            unsigned char l[288 + 30];
            memset(l, 8, 144), memset(l + 144, 9, 112), memset(l + 256, 7, 24), memset(l + 280, 8, 8);
            memset(l + 288, 5, 30);
            struct huffman lit, dist;
            huffman(&lit, l, 288), huffman(&dist, l + 288, 30);
            if (block(&b, &lit, &dist, out, &n) < 0) return n;
        } else if (type == 2) {
            struct huffman lit, dist;
            if (dynamic(&b, &lit, &dist) < 0 || block(&b, &lit, &dist, out, &n) < 0) return n;
        } else
            return n;
        if (last) return n;
    }
}

/* The `<code_set_name>` among the header lines of a charmap's text (before its `CHARMAP` line). */
static void code_set_name(struct names *s, const char *text, size_t len) {
    const char *end = text + len;
    for (const char *line = text; line < end;) {
        const char *nl = memchr(line, '\n', (size_t)(end - line));
        const char *stop = nl ? nl : end;
        size_t n = (size_t)(stop - line);
        if (n >= 7 && memcmp(line, "CHARMAP", 7) == 0) return;
        if (n >= 15 && memcmp(line, "<code_set_name>", 15) == 0) {
            const char *v = line + 15;
            while (v < stop && (*v == ' ' || *v == '\t')) v++;
            const char *w = v;
            while (w < stop && *w != ' ' && *w != '\t' && *w != '\r') w++;
            if (w > v) {
                char name[256];
                snprintf(name, sizeof name, "%.*s", (int)(w - v), v);
                add(s, name, NULL);
            }
            return;
        }
        line = stop + 1;
    }
}

/* One charmap file's header text, plain or gzip-compressed, into out (HEADER_MAX bytes); returns its length. */
static size_t charmap_header(const char *file, int gz, unsigned char *out) {
    char path[PATH_MAX];
    struct stat st;
    snprintf(path, sizeof path, CHARMAP_DIR "/%s", file);
    /* O_NONBLOCK: a FIFO in the directory is skipped, not waited on; only a regular file is read. */
    int fd = open(path, O_RDONLY | O_CLOEXEC | O_NONBLOCK);
    if (fd < 0) return 0;
    size_t n = 0;
    if (fstat(fd, &st) != 0 || !S_ISREG(st.st_mode)) {
        /* nothing to read */
    } else if (!gz) {
        while (n < HEADER_MAX) {
            ssize_t r = read(fd, out + n, HEADER_MAX - n);
            if (r < 0 && errno == EINTR) continue;
            if (r <= 0) break;
            n += (size_t)r;
        }
    } else if (st.st_size > 0) {
        void *m = mmap(NULL, (size_t)st.st_size, PROT_READ, MAP_PRIVATE, fd, 0);
        if (m != MAP_FAILED) {
            n = gunzip_prefix(m, (size_t)st.st_size, out);
            munmap(m, (size_t)st.st_size);
        }
    }
    close(fd);
    return n;
}

/* `-m`: each charmap file's name without `.gz`/`.bz2`, and the `<code_set_name>` of a plain or gzip one. */
static int list_charmaps(void) {
    DIR *d = opendir(CHARMAP_DIR);
    if (!d) {
        fprintf(stderr, "locale: [error] cannot read character map directory `%s': %s\n", CHARMAP_DIR,
                strerror(errno));
        return 1;
    }
    static unsigned char text[HEADER_MAX];
    struct names s = {0};
    struct dirent *e;
    while ((e = readdir(d))) {
        if (strcmp(e->d_name, ".") == 0 || strcmp(e->d_name, "..") == 0) continue;
        char name[NAME_MAX + 1];
        snprintf(name, sizeof name, "%s", e->d_name);
        size_t len = strlen(name);
        int gz = 0;
        if (len > 3 && strcmp(name + len - 3, ".gz") == 0) name[len - 3] = 0, gz = 1;
        else if (len > 4 && strcmp(name + len - 4, ".bz2") == 0) {
            /* Listed by its name only: libniji reads no bzip2 data (docs/differences-from-glibc.md). */
            name[len - 4] = 0;
            add(&s, name, NULL);
            continue;
        }
        add(&s, name, NULL);
        code_set_name(&s, (const char *)text, charmap_header(e->d_name, gz, text));
    }
    closedir(d);
    qsort(s.v, s.n, sizeof *s.v, by_collation);
    for (size_t i = 0; i < s.n; i++) puts(s.v[i].name);
    return 0;
}

static void try_set(int cat, const char *name) {
    if (!setlocale(cat, ""))
        fprintf(stderr, "locale: Cannot set %s to default locale: %s\n", name, strerror(errno));
}

static const char usage_text[] =
    "Usage: locale [-ckv?V] [--category-name] [--keyword-name] [--verbose] [--help]\n"
    "            [--usage] [--version] NAME\n"
    "  or:  locale [OPTION...] [-a|-m]\n";

static const char help_text[] =
    "Usage: locale [OPTION...] NAME\n"
    "  or:  locale [OPTION...] [-a|-m]\n"
    "Get locale-specific information.\n"
    "\n"
    " System information:\n"
    "  -a, --all-locales          Write names of available locales\n"
    "  -m, --charmaps             Write names of available charmaps\n"
    "\n"
    " Modify output format:\n"
    "  -c, --category-name        Write names of selected categories\n"
    "  -k, --keyword-name         Write names of selected keywords\n"
    "  -v, --verbose              Print more information\n"
    "\n"
    "  -?, --help                 Give this help list\n"
    "      --usage                Give a short usage message\n"
    "  -V, --version              Print program version\n";

static int bad(const char *fmt, const char *what) {
    fprintf(stderr, "locale: ");
    fprintf(stderr, fmt, what);
    fprintf(stderr, "\nTry `locale --help' or `locale --usage' for more information.\n");
    return 64;
}

int main(int argc, char **argv) {
    static const struct {
        const char *name;
        char opt;
    } longs[] = {
        {"all-locales", 'a'}, {"charmaps", 'm'}, {"category-name", 'c'}, {"keyword-name", 'k'},
        {"verbose", 'v'},     {"help", '?'},     {"usage", 'u'},         {"version", 'V'},
    };
    int all = 0, charmaps = 0, i = 1;
    for (; i < argc && argv[i][0] == '-' && argv[i][1]; i++) {
        const char *a = argv[i];
        if (strcmp(a, "--") == 0) {
            i++;
            break;
        }
        char opts[2] = {0, 0};
        const char *run = a + 1;
        if (a[1] == '-') {
            size_t len = strlen(a + 2), hits = 0;
            for (size_t j = 0; j < sizeof longs / sizeof longs[0]; j++)
                if (strncmp(longs[j].name, a + 2, len) == 0) {
                    if (strlen(longs[j].name) == len) {
                        hits = 1, opts[0] = longs[j].opt;
                        break;
                    }
                    hits++, opts[0] = longs[j].opt;
                }
            if (hits != 1) return bad(hits ? "option '%s' is ambiguous" : "unrecognized option '%s'", a);
            run = opts;
        }
        for (; *run; run++) switch (*run) {
            case 'a': all = 1; break;
            case 'm': charmaps = 1; break;
            case 'c': opt_c = 1; break;
            case 'k': opt_k = 1; break;
            case 'v': opt_v = 1; break;
            case '?': fputs(help_text, stdout); return 0;
            case 'u': fputs(usage_text, stdout); return 0;
            case 'V': puts("locale (libniji) 2.39"); return 0;
            default: {
                char c[2] = {*run, 0};
                return bad("invalid option -- '%s'", c);
            }
            }
    }
    try_set(LC_CTYPE, "LC_CTYPE");
    try_set(LC_MESSAGES, "LC_MESSAGES");
    if (all) {
        try_set(LC_COLLATE, "LC_COLLATE");
        list_locales();
        return 0;
    }
    if (charmaps) {
        try_set(LC_COLLATE, "LC_COLLATE");
        return list_charmaps();
    }
    try_set(LC_ALL, "LC_ALL");
    if (i >= argc) {
        show_environment();
        return 0;
    }
    /* The first unknown name ends the run (probe: `decimal_point nosuch grouping` prints only the first). */
    for (; i < argc; i++)
        if (!show(argv[i])) {
            fflush(stdout);
            fprintf(stderr, "locale: unknown name \"%s\"\n", argv[i]);
            return 1;
        }
    return 0;
}
