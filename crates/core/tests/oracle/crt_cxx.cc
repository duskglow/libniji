// A C++ program with static constructors and destructors, linked by a plain `g++`
// (crates/core/tests/crt_products.rs, SL-CRT): the namespace-scope objects are built before
// main and destroyed after it, in reverse, and the function-local one is destroyed first.
#include <cstdio>
#include <iostream>

struct Noisy {
    const char *name;
    explicit Noisy(const char *n) : name(n) { std::printf("ctor %s\n", name); }
    ~Noisy() { std::printf("dtor %s\n", name); }
};

static Noisy first("first");
static Noisy second("second");

int main() {
    std::cout << "main" << std::endl;
    static Noisy local("local");
    return 0;
}
