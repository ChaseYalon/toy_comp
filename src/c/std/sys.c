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
ToyPtr toy_sys_get_argv() {
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

ToyPtr toy_sys_get_os_name() {
    #ifdef _WIN32
        char* s = "windows";
        return toy_malloc((ToyPtr) s);
    #else
        char* s = "linux";
        return toy_malloc((ToyPtr) s);
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
int64_t toy_sys_is_little_endian() {
    uint16_t num = 0x1;
    char* byte = (char*)&num;
    if (byte[0] == 1) {
        return 1; // little-endian
    } else {
        return 0; // big-endian
    }
}

#include <stdint.h>
#include "../builtins.h"
#include <string.h>
#include <stdlib.h>
#ifdef _WIN32
#include <windows.h>
#else
#include <unistd.h>
#include <sys/wait.h>
#endif

int64_t toy_sys_invoke(ToyPtr code, ToyPtr args){
    if (code * args == 0){
        //error case
        return 1;
    }
    const char* exe = code ? (const char*)code : "./toy_comp.exe";

    #ifdef _WIN32
    STARTUPINFOA si;
    PROCESS_INFORMATION pi;
    ZeroMemory(&si, sizeof(si));
    si.cb = sizeof(si);
    ZeroMemory(&pi, sizeof(pi));

    int64_t nargs = args ? toy_arrlen(args) : 0;
    size_t cmdlen = strlen(exe) + 1;
    for (int64_t i = 0; i < nargs; ++i) {
        char* a = (char*) (intptr_t) toy_read_from_arr(args, i);
        if (a) cmdlen += 3 + strlen(a); // space + quotes + arg
    }

    char* cmdline = (char*) META_MALLOC(cmdlen + 1);
    if (!cmdline) return -1;
    cmdline[0] = '\0';
    strcat(cmdline, exe);

    for (int64_t i = 0; i < nargs; ++i) {
        char* a = (char*) (intptr_t) toy_read_from_arr(args, i);
        if (!a) continue;
        strcat(cmdline, " \"");
        strcat(cmdline, a);
        strcat(cmdline, "\"");
    }

    DWORD exitCode = (DWORD)-1;
    if (CreateProcessA(NULL, cmdline, NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi)) {
        WaitForSingleObject(pi.hProcess, INFINITE);
        GetExitCodeProcess(pi.hProcess, &exitCode);
        CloseHandle(pi.hProcess);
        CloseHandle(pi.hThread);
    }

    toy_free(cmdline);
    return (int64_t) exitCode;

    #else

    int64_t nargs = args ? toy_arrlen(args) : 0;
    pid_t pid = fork();
    if (pid < 0) {
        return -1;
    }
    if (pid == 0) {
        // child
        char** argv = (char**) malloc((nargs + 2) * sizeof(char*));
        if (!argv) _exit(127);
        argv[0] = (char*) exe;
        for (int64_t i = 0; i < nargs; ++i) {
            argv[i+1] = (char*) (intptr_t) toy_read_from_arr(args, i);
        }
        argv[nargs+1] = NULL;
        execv(argv[0], argv);
        _exit(127);
    } else {
        int status = 0;
        waitpid(pid, &status, 0);
        if (WIFEXITED(status)) {
            return (int64_t) WEXITSTATUS(status);
        } else {
            return -1;
        }
    }

    #endif
}