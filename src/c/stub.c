#include <stdint.h>
#include <stdlib.h>
extern int64_t user_main();

//sets environment variables, everything is in debug mode by default
void _SetDebug_env() {
    #ifdef _WIN32
        _putenv_s("TOY_DEBUG", "TRUE");
    #endif
    #ifdef __LINUX__
        setenv("TOY_DEBUG", "TRUE", 1);
    #endif
}

int main(){
    _SetDebug_env();
    int res = (int) user_main();
    return res;
}