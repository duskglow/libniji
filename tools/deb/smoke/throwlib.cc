// Smoke library for the reference packages (tools/deb/root-system.sh): a shared object linked
// against the system's libstdc++.so.6 that throws a standard exception for its caller to catch, so
// the exception crosses a shared-library boundary, and a function-local static (a unique object).
#include <stdexcept>
#include <string>

std::string &throwlib_name() {
    static std::string name("throwlib");
    return name;
}

int throwlib_check(int x) {
    if (x > 1) {
        throw std::out_of_range(throwlib_name() + ": value " + std::to_string(x));
    }
    return x;
}
