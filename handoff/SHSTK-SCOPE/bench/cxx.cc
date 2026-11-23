// SHSTK-SCOPE: C++ throw-catch through 10 frames, N times. Usage: cxx N
#include <cstdlib>
static volatile int sink;
__attribute__((noinline)) static void f(int d)
{
	if (d == 0)
		throw d;
	f(d - 1);
	sink++;
}
int main(int argc, char **argv)
{
	long n = argc > 1 ? atol(argv[1]) : 0;
	long c = 0;
	for (long i = 0; i < n; i++) {
		try {
			f(10);
		} catch (int) {
			c++;
		}
	}
	return c == n ? 0 : 1;
}
