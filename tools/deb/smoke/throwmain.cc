// Smoke program for the reference packages (tools/deb/root-system.sh): catches an exception thrown
// in libthrowlib.so (smoke/throwlib.cc), both linked against the system's libstdc++.so.6.
#include <iostream>
#include <stdexcept>
#include <string>

std::string &throwlib_name();
int throwlib_check(int x);

int main() {
    try {
        throwlib_check(3);
    } catch (const std::out_of_range &e) {
        std::cout << "throwmain: caught: " << e.what() << " (" << throwlib_name() << ")" << std::endl;
        return 0;
    }
    std::cout << "throwmain: not thrown" << std::endl;
    return 1;
}
