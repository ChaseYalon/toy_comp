#include <stdint.h>
#include <stdlib.h>
#include "../builtins.h"
#include <stdio.h>
#include <string.h>
void toy_sys_exit(int64_t code) {
    exit(code);
}


#if defined(_WIN32) || defined(_WIN64)
#include <windows.h>
int64_t toy_sys_get_pid() {
    return (int64_t)GetCurrentProcessId();
}
#else
#include <unistd.h>
int64_t toy_sys_get_pid() {
    return (int64_t)getpid();
}
#endif

int64_t toy_sys_get_argc() {
    return GLOBAL_ARGC;
}
int64_t toy_sys_get_argv() {
    int64_t arr = toy_malloc_arr(GLOBAL_ARGC, 4, 1); // 4 is str[]
    if (arr == 0){
        fprintf(stderr, "[ERROR] toy_sys_get_argv failed to allocate argv array\n");
        abort();
    }
    for (int64_t i = 0; i < GLOBAL_ARGC; i++) {
        char* c_str = GLOBAL_ARGV[i];
        toy_write_to_arr(arr, (int64_t)c_str, i, 4);
    }
    return arr;
}

int64_t toy_sys_get_os_name() {
    #ifdef _WIN32
        char* s = "windows";
        return toy_malloc((int64_t) s);
    #else
        char* s = "linux";
        return toy_malloc((int64_t) s);
    #endif
}

int64_t toy_sys_get_core_count() {
    #ifdef _WIN32
        #include <windows.h>
        SYSTEM_INFO sysinfo;
        GetSystemInfo(&sysinfo);
        return (int64_t)sysinfo.dwNumberOfProcessors;
    #else
        #include <unistd.h>
        return (int64_t)sysconf(_SC_NPROCESSORS_ONLN);
    #endif
}