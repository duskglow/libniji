/* P10 for slice 17 (plan/headers.md section 3): the obstack macros and the sys/queue.h macros, evaluated by a
   program that is built twice (against the reference's headers and against libniji's) and prints the same text.
   No spec clause: header macros (GNU C Library manual, Obstacks; queue(7) and list(3), slist(3), stailq(3),
   tailq(3), circleq(3)). */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <obstack.h>
#include <sys/queue.h>

#define obstack_chunk_alloc malloc
#define obstack_chunk_free free

struct item { int v; LIST_ENTRY (item) l; SLIST_ENTRY (item) s; STAILQ_ENTRY (item) q; TAILQ_ENTRY (item) t; CIRCLEQ_ENTRY (item) c; };
LIST_HEAD (lh, item);
SLIST_HEAD (sh, item);
STAILQ_HEAD (qh, item);
TAILQ_HEAD (th, item);
CIRCLEQ_HEAD (ch, item);

int main (void)
{
  struct obstack ob;
  obstack_init (&ob);
  char *a = obstack_alloc (&ob, 10);
  char *b = obstack_copy (&ob, "hello", 5);
  char *c = obstack_copy0 (&ob, "world", 5);
  printf ("alloc/copy %d %s %s %d\n", (int) (b - a) >= 10, strncmp (b, "hello", 5) == 0 ? "ok" : "bad", c, (int) strlen (c));
  for (int i = 0; i < 5000; i++)
    obstack_1grow (&ob, (char) ('a' + i % 26));
  obstack_int_grow (&ob, 77);
  obstack_ptr_grow (&ob, a);
  int sz = obstack_object_size (&ob);
  void *base = obstack_base (&ob);
  char *big = obstack_finish (&ob);
  printf ("grow size %d base-is-object %d aligned %d first %c last-char-ok %d\n", sz, base == (void *) big,
          (int) (((unsigned long) obstack_next_free (&ob) & obstack_alignment_mask (&ob)) == 0), big[0], big[4999] == 'a' + 4999 % 26);
  obstack_blank (&ob, 100000);
  obstack_blank_fast (&ob, 0);
  printf ("room-after-blank %d size %d\n", obstack_room (&ob) >= 0, obstack_object_size (&ob));
  void *e = obstack_finish (&ob);
  obstack_free (&ob, e);
  obstack_free (&ob, NULL);

  struct item it[6];
  for (int i = 0; i < 6; i++) it[i].v = i;
  struct lh L; LIST_INIT (&L);
  LIST_INSERT_HEAD (&L, &it[0], l); LIST_INSERT_AFTER (&it[0], &it[1], l); LIST_INSERT_BEFORE (&it[1], &it[2], l);
  LIST_REMOVE (&it[0], l);
  struct item *p; printf ("list");
  LIST_FOREACH (p, &L, l) printf (" %d", p->v);
  printf (" empty %d first %d\n", LIST_EMPTY (&L) != 0, LIST_FIRST (&L)->v);
  struct sh S; SLIST_INIT (&S);
  SLIST_INSERT_HEAD (&S, &it[0], s); SLIST_INSERT_AFTER (&it[0], &it[1], s); SLIST_INSERT_HEAD (&S, &it[2], s);
  SLIST_REMOVE (&S, &it[1], item, s); SLIST_REMOVE_HEAD (&S, s);
  printf ("slist");
  SLIST_FOREACH (p, &S, s) printf (" %d", p->v);
  printf ("\n");
  struct qh Q, Q2; STAILQ_INIT (&Q); STAILQ_INIT (&Q2);
  STAILQ_INSERT_TAIL (&Q, &it[0], q); STAILQ_INSERT_TAIL (&Q, &it[1], q); STAILQ_INSERT_HEAD (&Q, &it[2], q);
  STAILQ_INSERT_AFTER (&Q, &it[2], &it[3], q); STAILQ_INSERT_TAIL (&Q2, &it[4], q);
  STAILQ_CONCAT (&Q, &Q2); STAILQ_REMOVE (&Q, &it[1], item, q);
  printf ("stailq");
  STAILQ_FOREACH (p, &Q, q) printf (" %d", p->v);
  printf (" q2-empty %d\n", STAILQ_EMPTY (&Q2) != 0);
  struct th T, T2; TAILQ_INIT (&T); TAILQ_INIT (&T2);
  TAILQ_INSERT_TAIL (&T, &it[0], t); TAILQ_INSERT_HEAD (&T, &it[1], t); TAILQ_INSERT_AFTER (&T, &it[1], &it[2], t);
  TAILQ_INSERT_BEFORE (&it[0], &it[3], t); TAILQ_INSERT_TAIL (&T2, &it[4], t); TAILQ_CONCAT (&T, &T2, t); TAILQ_REMOVE (&T, &it[2], t);
  printf ("tailq last %d prev-of-last %d fwd", TAILQ_LAST (&T, th)->v, TAILQ_PREV (TAILQ_LAST (&T, th), th, t)->v);
  TAILQ_FOREACH (p, &T, t) printf (" %d", p->v);
  printf (" rev");
  TAILQ_FOREACH_REVERSE (p, &T, th, t) printf (" %d", p->v);
  printf ("\n");
  struct ch C; CIRCLEQ_INIT (&C);
  CIRCLEQ_INSERT_TAIL (&C, &it[0], c); CIRCLEQ_INSERT_HEAD (&C, &it[1], c); CIRCLEQ_INSERT_AFTER (&C, &it[1], &it[2], c);
  CIRCLEQ_INSERT_BEFORE (&C, &it[0], &it[3], c); CIRCLEQ_REMOVE (&C, &it[2], c);
  printf ("circleq fwd");
  CIRCLEQ_FOREACH (p, &C, c) printf (" %d", p->v);
  printf (" rev");
  CIRCLEQ_FOREACH_REVERSE (p, &C, c) printf (" %d", p->v);
  printf (" loopnext-of-last %d loopprev-of-first %d\n", CIRCLEQ_LOOP_NEXT (&C, CIRCLEQ_LAST (&C), c)->v, CIRCLEQ_LOOP_PREV (&C, CIRCLEQ_FIRST (&C), c)->v);
  return 0;
}
