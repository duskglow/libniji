/* validation/smoke/dlclose/reloadmod.c: a Lua C module (smoke program lua_reload). lua_reload requires it in
   a fresh Lua state 50 times; each lua_close unloads it (Lua's loader calls dlclose). Its counter is static
   data: when the unload really unmapped it, every load starts from 0 again. The module calls back into the
   Lua library, whose symbols the program exports (-Wl,-E), so each load also resolves symbols in the program. */
#include <stdio.h>

#include "lauxlib.h"
#include "lua.h"

static int counter;
static int loads;

__attribute__((constructor)) static void loaded(void) { loads++; }
__attribute__((destructor)) static void unloaded(void) { printf("reloadmod: unloaded (loads %d, counter %d)\n", loads, counter); }

static int bump(lua_State *L) {
    lua_pushinteger(L, ++counter);
    return 1;
}

static int loads_seen(lua_State *L) {
    lua_pushinteger(L, loads);
    return 1;
}

static int concat(lua_State *L) {
    luaL_Buffer b;
    int n = lua_gettop(L);
    luaL_buffinit(L, &b);
    for (int i = 1; i <= n; i++) {
        lua_pushvalue(L, i);
        luaL_addvalue(&b);
    }
    luaL_pushresult(&b);
    return 1;
}

int luaopen_reloadmod(lua_State *L) {
    static const luaL_Reg funcs[] = {{"bump", bump}, {"loads", loads_seen}, {"concat", concat}, {NULL, NULL}};
    luaL_newlib(L, funcs);
    return 1;
}
