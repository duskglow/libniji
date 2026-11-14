#include <time.h>
/* Start-up, plus one clock call (HEAP-FIRST-USE-8): the vDSO lookup and liella_machine_vcall3 used to run
   at every start-up (the heap's first slow path read the thread's node through the vDSO getcpu). Since
   D-3.20 only a process allowed several NUMA nodes asks the node, so on a one-node build host start-up
   no longer traces them, and they fell out of the hot group although a program's first time,
   gettimeofday or getcpu runs them. Traced here, they keep their start-up place on every build host. */
static time_t (*volatile time_fn)(time_t *) = time;
int main(void) { return time_fn(0) > 0 ? 0 : 1; }
