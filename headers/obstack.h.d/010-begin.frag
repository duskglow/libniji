@@owns obstack_begin obstack_specify_allocation obstack_specify_allocation_with_arg obstack_chunkfun obstack_freefun obstack_empty_p obstack_memory_used
/* Starting an obstack with a chunk size, alignment and allocation functions of the program's choice, replacing
   those functions later, and asking about its state (GNU C Library manual, Obstacks; R-H7: dwarves). A size or
   alignment of 0 takes the default. An obstack is empty when its current chunk is its first one and that chunk holds
   no object: the next free byte is the chunk's first aligned one, the address _obstack_begin starts from. */
@@if obstack_begin
# define obstack_begin(__h, __size) _obstack_begin ((__h), (__size), 0, (void *(*) (long int)) obstack_chunk_alloc, (void (*) (void *)) obstack_chunk_free)
# define obstack_specify_allocation(__h, __size, __align, __chunkfun, __freefun) _obstack_begin ((__h), (__size), (__align), (void *(*) (long int)) (__chunkfun), (void (*) (void *)) (__freefun))
# define obstack_specify_allocation_with_arg(__h, __size, __align, __chunkfun, __freefun, __arg) _obstack_begin_1 ((__h), (__size), (__align), (void *(*) (void *, long int)) (__chunkfun), (void (*) (void *, void *)) (__freefun), (__arg))
# define obstack_chunkfun(__h, __f) ((__h)->chunkfun = (__typeof__ ((__h)->chunkfun)) (__f))
# define obstack_freefun(__h, __f) ((__h)->freefun = (__typeof__ ((__h)->freefun)) (__f))
# define obstack_empty_p(__h) __extension__ ({ __typeof__ (__h) __o = (__h); (__o->chunk->prev == 0 && __o->next_free == (char *) (((__PTRDIFF_TYPE__) __o->chunk->contents + __o->alignment_mask) & ~ (__PTRDIFF_TYPE__) __o->alignment_mask)); })
# define obstack_memory_used(__h) _obstack_memory_used (__h)
#endif
