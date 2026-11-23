/* charmap-header.h: the header text of a charmap file, plain or gzip-compressed, for the utilities that list
 * charmap names (`locale -m`, `iconv -l`; spec §31.6, §31.7). One in-program inflater for both: libc.so.6
 * exports no inflater or listing helper (coordinator rulings #6246, #6261), so each program carries this code.
 * The includer defines CHARMAP_DIR and includes it once; every function is static. */
#ifndef NIJI_UTIL_CHARMAP_HEADER_H
#define NIJI_UTIL_CHARMAP_HEADER_H

#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>

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

/* One compressed block into out[*n..cap): 0 at its end-of-block code, -1 when it stops early. */
static int block(struct bits *b, const struct huffman *lit, const struct huffman *dist, unsigned char *out,
                 size_t *n, size_t cap) {
    for (;;) {
        int sym = decode(lit, b);
        if (sym < 0) return -1;
        if (sym < 256) {
            if (*n >= cap) return -1;
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
            if (*n >= cap) return -1;
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

/* The text at the start of a gzip file `data`, at most cap bytes (cap <= HEADER_MAX) into out; returns its length. */
static size_t gunzip_prefix(const unsigned char *data, size_t size, unsigned char *out, size_t cap) {
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
                if (b.pos >= b.n || n >= cap) return n;
                out[n++] = b.p[b.pos++];
            }
        } else if (type == 1) {
            unsigned char l[288 + 30];
            memset(l, 8, 144), memset(l + 144, 9, 112), memset(l + 256, 7, 24), memset(l + 280, 8, 8);
            memset(l + 288, 5, 30);
            struct huffman lit, dist;
            huffman(&lit, l, 288), huffman(&dist, l + 288, 30);
            if (block(&b, &lit, &dist, out, &n, cap) < 0) return n;
        } else if (type == 2) {
            struct huffman lit, dist;
            if (dynamic(&b, &lit, &dist) < 0 || block(&b, &lit, &dist, out, &n, cap) < 0) return n;
        } else
            return n;
        if (last) return n;
    }
}

/* One charmap file's header text, plain or gzip-compressed, into out (HEADER_MAX bytes); returns its length. At
   most `in_max` bytes of the file are read and at most `out_max` bytes of text kept (both capped at HEADER_MAX
   for a plain file): `iconv -l` reads what iconv_open's header scan reads, `locale -m` up to HEADER_MAX. */
static size_t charmap_header(const char *file, int gz, unsigned char *out, size_t in_max, size_t out_max) {
    char path[PATH_MAX];
    struct stat st;
    snprintf(path, sizeof path, CHARMAP_DIR "/%s", file);
    /* O_NONBLOCK: a FIFO in the directory is skipped, not waited on; only a regular file is read. */
    int fd = open(path, O_RDONLY | O_CLOEXEC | O_NONBLOCK);
    if (fd < 0) return 0;
    size_t n = 0;
    if (out_max > HEADER_MAX) out_max = HEADER_MAX;
    if (fstat(fd, &st) != 0 || !S_ISREG(st.st_mode)) {
        /* nothing to read */
    } else if (!gz) {
        unsigned char *p = out, *end = out + (in_max < out_max ? in_max : out_max);
        while (p < end) {
            ssize_t r = read(fd, p, (size_t)(end - p));
            if (r < 0 && errno == EINTR) continue;
            if (r <= 0 || r > end - p) break;
            p += r;
        }
        n = (size_t)(p - out);
    } else if (st.st_size > 0) {
        void *m = mmap(NULL, (size_t)st.st_size, PROT_READ, MAP_PRIVATE, fd, 0);
        if (m != MAP_FAILED) {
            size_t size = (size_t)st.st_size < in_max ? (size_t)st.st_size : in_max;
            n = gunzip_prefix(m, size, out, out_max);
            munmap(m, (size_t)st.st_size);
        }
    }
    close(fd);
    return n;
}

#endif
