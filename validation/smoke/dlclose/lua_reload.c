/* validation/smoke/dlclose/lua_reload.c: smoke program lua_reload. Embeds the Lua library built from
   Debian's lua5.4 source and, 50 times over, opens a fresh Lua state, requires the C module reloadmod (Lua's
   package loader: dlopen and dlsym of luaopen_reloadmod), calls it, and closes the state (Lua unloads the
   module with dlclose). A second module name loaded with package.loadlib in the same state exercises an
   explicit dlsym by name. Every round prints one line; smoke.sh compares the output with the reference build's.

     lua_reload DIR     (DIR holds reloadmod.so) */
#include <stdio.h>
#include <string.h>

#include "lauxlib.h"
#include "lua.h"
#include "lualib.h"

static const char script[] =
    "local dir = ...\n"
    "package.cpath = dir .. '/?.so'\n"
    "local m = require 'reloadmod'\n"
    "local a, b = m.bump(), m.bump()\n"
    "local f = assert(package.loadlib(dir .. '/reloadmod.so', 'luaopen_reloadmod'))\n"
    "local again = f()\n"
    "local c = again.bump()\n"
    "return string.format('counter %d %d %d, loads %d, %s', a, b, c, m.loads(), m.concat('x', 1, 'y'))\n";

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: lua_reload DIR\n");
        return 2;
    }
    setvbuf(stdout, NULL, _IONBF, 0);
    for (int round = 1; round <= 50; round++) {
        lua_State *L = luaL_newstate();
        if (L == NULL) {
            printf("round %d: luaL_newstate failed\n", round);
            return 1;
        }
        luaL_openlibs(L);
        if (luaL_loadbuffer(L, script, strlen(script), "reload") != LUA_OK) {
            printf("round %d: load: %s\n", round, lua_tostring(L, -1));
            return 1;
        }
        lua_pushstring(L, argv[1]);
        if (lua_pcall(L, 1, 1, 0) != LUA_OK) {
            printf("round %d: error: %s\n", round, lua_tostring(L, -1));
            return 1;
        }
        printf("round %d: %s\n", round, lua_tostring(L, -1));
        lua_close(L);
    }
    printf("lua_reload: done\n");
    return 0;
}
