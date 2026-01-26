#include <stdint.h>
#ifdef _WIN32
    #include <windows.h>
#else
    #include <time.h>
#endif

int64_t toy_time_ms_since_unix_epoch() {
    #ifdef _WIN32
        FILETIME ft;
        GetSystemTimeAsFileTime(&ft);
        ULARGE_INTEGER uli;
        uli.LowPart = ft.dwLowDateTime;
        uli.HighPart = ft.dwHighDateTime;
        // Convert to milliseconds since Unix epoch
        return (uli.QuadPart / 10000) - 11644473600000ULL;
    #else 
        struct timespec ts;
        clock_gettime(CLOCK_REALTIME, &ts);
        return (int64_t)(ts.tv_sec) * 1000 + (ts.tv_nsec / 1000000);
    #endif
}
int64_t toy_time_current_year() {
    #ifdef _WIN32
        SYSTEMTIME st;
        GetLocalTime(&st);
        return (int64_t)st.wYear;
    #else
        time_t now = time(NULL);
        struct tm *tm_info = localtime(&now);
        return (int64_t)(tm_info->tm_year + 1900);
    #endif
}

int64_t toy_time_current_month() {
    #ifdef _WIN32
        SYSTEMTIME st;
        GetLocalTime(&st);
        return (int64_t)st.wMonth;  // 1 = January, 12 = December
    #else
        time_t now = time(NULL);
        struct tm tm_info;
        localtime_r(&now, &tm_info); // thread-safe
        return (int64_t)(tm_info.tm_mon + 1); // convert 0-based to 1-based
    #endif
}

int64_t toy_time_current_day() {
    #ifdef _WIN32
        SYSTEMTIME st;
        GetLocalTime(&st);
        return (int64_t)st.wDay;  // 1–31
    #else
        time_t now = time(NULL);
        struct tm tm_info;
        localtime_r(&now, &tm_info); // thread-safe
        return (int64_t)tm_info.tm_mday; // 1–31
    #endif
}


void toy_time_sleep(int64_t ms) {
    #ifdef _WIN32
        Sleep((DWORD)ms);
    #else
        struct timespec ts;
        ts.tv_sec = ms / 1000;
        ts.tv_nsec = (ms % 1000) * 1000000;
        nanosleep(&ts, NULL);
    #endif
}