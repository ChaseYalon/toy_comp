#include "../builtins.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <strings.h>
#ifdef _WIN32
#include <windows.h>
#include "../builtins.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#ifdef _WIN32
#include <windows.h>
#else
#include <errno.h>
#include <unistd.h>
#include <fcntl.h>
#include <string.h>
#endif

ToyPtr toy_fs_read_file(ToyPtr path) {
    const char *p = (const char*)path;
#ifdef _WIN32
    HANDLE h = CreateFileA(
        p,
        GENERIC_READ,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        NULL,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        NULL
    );
    if (h == INVALID_HANDLE_VALUE) {
        DWORD e = GetLastError();
        fprintf(stderr, "[ERROR] CreateFileA failed for '%s' (GetLastError=%lu)\n", p, (unsigned long)e);
        abort();
    }

    LARGE_INTEGER sz;
    if (!GetFileSizeEx(h, &sz)) {
        DWORD e = GetLastError();
        fprintf(stderr, "[ERROR] GetFileSizeEx failed for '%s' (GetLastError=%lu)\n", p, (unsigned long)e);
        CloseHandle(h);
        abort();
    }

    if (sz.QuadPart < 0 || (unsigned long long)sz.QuadPart > (unsigned long long)(SIZE_MAX - 1)) {
        fprintf(stderr, "[ERROR] file too large: '%s'\n", p);
        CloseHandle(h);
        abort();
    }

    size_t size = (size_t)sz.QuadPart;
    char* buffer = META_MALLOC(size + 1);
    if (!buffer) { CloseHandle(h); abort(); }

    DWORD got = 0;
    if (size > 0) {
        if (!ReadFile(h, buffer, (DWORD)size, &got, NULL)) {
            DWORD e = GetLastError();
            fprintf(stderr, "[ERROR] ReadFile failed for '%s' (GetLastError=%lu)\n", p, (unsigned long)e);
            CloseHandle(h);
            abort();
        }
    }
    CloseHandle(h);

    // Null-terminate after all bytes are read
    buffer[got] = '\0';
    return (ToyPtr)buffer;

#else
    FILE* f = fopen(p, "rb");
    if (!f) {
        fprintf(stderr, "[ERROR] fopen failed for '%s' (errno=%d: %s)\n", p, errno, strerror(errno));
        abort();
    }

    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    rewind(f);

    if (size < 0) {
        fclose(f);
        fprintf(stderr, "[ERROR] ftell failed for '%s'\n", p);
        abort();
    }

    char* buffer = META_MALLOC((size_t)size + 1);
    if (!buffer) { fclose(f); abort(); }

    size_t read = fread(buffer, 1, (size_t)size, f);
    fclose(f);

    // Null-terminate after all bytes are read
    buffer[read] = '\0';
    return (ToyPtr)buffer;
#endif
}
int64_t toy_fs_write_file(ToyPtr path, ToyPtr content) {
    const char* p = (const char*)path;
    const char* data = (const char*)content;

    HANDLE h = CreateFileA(
        p,
        GENERIC_WRITE,
        0,                      // no sharing
        NULL,
        CREATE_ALWAYS,          // overwrite if exists
        FILE_ATTRIBUTE_NORMAL,
        NULL
    );

    if (h == INVALID_HANDLE_VALUE) {
        DWORD e = GetLastError();
        fprintf(stderr, "[ERROR] CreateFileA(write) failed '%s' (GetLastError=%lu)\n",
                p, (unsigned long)e);
        return -1;
    }

    DWORD written = 0;
    DWORD len = (DWORD)strlen(data);

    if (!WriteFile(h, data, len, &written, NULL)) {
        DWORD e = GetLastError();
        fprintf(stderr, "[ERROR] WriteFile failed '%s' (GetLastError=%lu)\n",
                p, (unsigned long)e);
        CloseHandle(h);
        abort();
    }

    CloseHandle(h);
    return 0;
}
void toy_fs_append_file(ToyPtr path, ToyPtr content) {
    const char* p = (const char*)path;
    const char* data = (const char*)content;

    HANDLE h = CreateFileA(
        p,
        FILE_APPEND_DATA,
        FILE_SHARE_READ,
        NULL,
        OPEN_ALWAYS,            // create if missing
        FILE_ATTRIBUTE_NORMAL,
        NULL
    );

    if (h == INVALID_HANDLE_VALUE) {
        DWORD e = GetLastError();
        fprintf(stderr, "[ERROR] CreateFileA(append) failed '%s' (GetLastError=%lu)\n",
                p, (unsigned long)e);
        abort();
    }

    DWORD written = 0;
    DWORD len = (DWORD)strlen(data);

    if (!WriteFile(h, data, len, &written, NULL)) {
        DWORD e = GetLastError();
        fprintf(stderr, "[ERROR] Append WriteFile failed '%s' (GetLastError=%lu)\n",
                p, (unsigned long)e);
        CloseHandle(h);
        abort();
    }

    CloseHandle(h);
}
int64_t toy_fs_file_size(ToyPtr path) {
    const char* p = (const char*)path;
    HANDLE h = CreateFileA(
        p, GENERIC_READ,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL
    );
    if (h == INVALID_HANDLE_VALUE) return -1;
    LARGE_INTEGER sz;
    if (!GetFileSizeEx(h, &sz)) { CloseHandle(h); return -1; }
    CloseHandle(h);
    return (int64_t)sz.QuadPart;
}
#else
#include <string.h>
#include <errno.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
ToyPtr toy_fs_read_file(ToyPtr path){
    const char* p = (const char*)path;
    FILE* fptr = fopen(p, "rb");
    if (!fptr){
        fprintf(stderr, "[ERROR] fopen failed for '%s' (errno=%d: %s)\n", p, errno, strerror(errno));
        abort();
    }

    fseek(fptr, 0, SEEK_END);
    long size = ftell(fptr);
    rewind(fptr);

    char *buffer = META_MALLOC((size_t)size + 1);
    if (!buffer){
        fprintf(stderr, "[ERROR] alloc failed (%ld bytes)\n", size + 1);
        fclose(fptr);
        abort();
    }

    size_t read = fread(buffer, 1, (size_t)size, fptr);
    fclose(fptr);
    buffer[read] = '\0';
    return (ToyPtr)buffer;
}
static void _write_all_or_die(int fd, const char* p, const char* data, size_t len) {
    size_t off = 0;
    while (off < len) {
        ssize_t n = write(fd, data + off, len - off);
        if (n < 0) {
            if (errno == EINTR) continue;
            fprintf(stderr, "[ERROR] write failed '%s' (errno=%d: %s)\n", p, errno, strerror(errno));
            close(fd);
            abort();
        }
        off += (size_t)n;
    }
}

ToyPtr toy_fs_write_file(ToyPtr path, ToyPtr content) {
    const char* p = (const char*)path;
    const char* data = (const char*)content;
    size_t len = strlen(data);

    int fd = open(p, O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (fd < 0) {
        fprintf(stderr, "[ERROR] open(write) failed '%s' (errno=%d: %s)\n", p, errno, strerror(errno));
        abort();
    }

    _write_all_or_die(fd, p, data, len);

    if (close(fd) != 0) {
        fprintf(stderr, "[ERROR] close failed '%s' (errno=%d: %s)\n", p, errno, strerror(errno));
        abort();
    }

    return 0;
}

ToyPtr toy_fs_append_file(ToyPtr path, ToyPtr content) {
    const char* p = (const char*)path;
    const char* data = (const char*)content;
    size_t len = strlen(data);

    int fd = open(p, O_WRONLY | O_CREAT | O_APPEND, 0644);
    if (fd < 0) {
        fprintf(stderr, "[ERROR] open(append) failed '%s' (errno=%d: %s)\n", p, errno, strerror(errno));
        abort();
    }

    _write_all_or_die(fd, p, data, len);

    if (close(fd) != 0) {
        fprintf(stderr, "[ERROR] close failed '%s' (errno=%d: %s)\n", p, errno, strerror(errno));
        abort();
    }

    return 0;
}
int64_t toy_fs_file_size(ToyPtr path) {
    const char* p = (const char*)path;
    FILE* f = fopen(p, "rb");
    if (!f) return -1;
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    fclose(f);
    return (int64_t)sz;
}
#endif
#ifdef _WIN32
//the delta between this code and the linux version is so astounding, Microsoft is worth however many trillions and yet their API looks like a 3 year old took an OS course.
int64_t _count_files(const char* path) {
    WIN32_FIND_DATAA ffd;
    HANDLE hFind;
    int64_t count = 0;

    // Build search pattern: "path\*"
    char pattern[MAX_PATH];
    snprintf(pattern, MAX_PATH, "%s\\*", path);

    hFind = FindFirstFileA(pattern, &ffd);
    if (hFind == INVALID_HANDLE_VALUE) {
        fprintf(stderr, "[ERROR] Could not open directory %s (error %lu)\n", path, GetLastError());
        abort();
    }

    do {
        if (!(ffd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY)) {
            count++;
        }
    } while (FindNextFileA(hFind, &ffd));

    FindClose(hFind);
    return count;
}

char** _list_files(const char* path) {
    WIN32_FIND_DATAA ffd;
    HANDLE hFind;

    char pattern[MAX_PATH];
    snprintf(pattern, MAX_PATH, "%s\\*", path);

    hFind = FindFirstFileA(pattern, &ffd);
    if (hFind == INVALID_HANDLE_VALUE) {
        fprintf(stderr, "[ERROR] Could not open directory %s (error %lu)\n", path, GetLastError());
        abort();
    }

    int64_t count = _count_files(path);
    char** arr = malloc(sizeof(char*) * count);
    int64_t i = 0;

    do {
        if (!(ffd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY)) {
            arr[i] = _strdup(ffd.cFileName);
            i++;
        }
    } while (FindNextFileA(hFind, &ffd));

    FindClose(hFind);
    return arr;
}

int64_t _count_dirs(const char* path) {
    WIN32_FIND_DATAA ffd;
    HANDLE hFind;
    int64_t count = 0;

    char pattern[MAX_PATH];
    snprintf(pattern, MAX_PATH, "%s\\*", path);

    hFind = FindFirstFileA(pattern, &ffd);
    if (hFind == INVALID_HANDLE_VALUE) {
        fprintf(stderr, "[ERROR] Could not open directory %s (error %lu)\n", path, GetLastError());
        abort();
    }

    do {
        if ((ffd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) &&
            strcmp(ffd.cFileName, ".") != 0 &&
            strcmp(ffd.cFileName, "..") != 0) {
            count++;
        }
    } while (FindNextFileA(hFind, &ffd));

    FindClose(hFind);
    return count;
}

char** _list_dirs(const char* path) {
    WIN32_FIND_DATAA ffd;
    HANDLE hFind;

    char pattern[MAX_PATH];
    snprintf(pattern, MAX_PATH, "%s\\*", path);

    hFind = FindFirstFileA(pattern, &ffd);
    if (hFind == INVALID_HANDLE_VALUE) {
        fprintf(stderr, "[ERROR] Could not open directory %s (error %lu)\n", path, GetLastError());
        abort();
    }

    int64_t count = _count_dirs(path);
    char** arr = malloc(sizeof(char*) * count);
    int64_t i = 0;

    do {
        if ((ffd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) &&
            strcmp(ffd.cFileName, ".") != 0 &&
            strcmp(ffd.cFileName, "..") != 0) {
            arr[i] = _strdup(ffd.cFileName);
            i++;
        }
    } while (FindNextFileA(hFind, &ffd));

    FindClose(hFind);
    return arr;
}
#else
#include <dirent.h>
int64_t _count_dirs(const char* path) {
    DIR *dir;
    struct dirent *ent;

    dir = opendir(path);
    if (dir == NULL) {
        fprintf(stderr, "[ERROR] Could not open directory %s", path);
        abort();
    }

    int64_t i = 0;
    while ((ent = readdir(dir)) != NULL) {
        if (ent->d_type == DT_DIR &&
            strcmp(ent->d_name, ".") != 0 &&
            strcmp(ent->d_name, "..") != 0) {
            i++;
        }
    }

    closedir(dir);
    return i;
}

char** _list_dirs(const char* path) {
    DIR *dir;
    struct dirent *ent;

    dir = opendir(path);
    if (dir == NULL) {
        fprintf(stderr, "[ERROR] Could not open directory %s", path);
        abort();
    }

    int64_t count = _count_dirs(path);
    char** arr = malloc(sizeof(char*) * count);
    int64_t i = 0;
    while ((ent = readdir(dir)) != NULL) {
        if (ent->d_type == DT_DIR &&
            strcmp(ent->d_name, ".") != 0 &&
            strcmp(ent->d_name, "..") != 0) {
            arr[i] = strdup(ent->d_name);
            i++;
        }
    }

    closedir(dir);
    return arr;
}
int64_t _count_files(const char* path){
    DIR *dir;
    struct dirent *ent;

    dir = opendir(path);
    if (dir == NULL) {
        fprintf(stderr, "[ERROR] Could not open director %s", path);
        abort();
    }

    int64_t i = 0;
    while ((ent = readdir(dir)) != NULL) {
        if (ent->d_type == DT_REG) {  // DT_REG = regular file
            i++;
        }
    }

    closedir(dir);
    return i;
}
char** _list_files(const char* path) {
    DIR *dir;
    struct dirent *ent;

    dir = opendir(path);
    if (dir == NULL) {
        fprintf(stderr, "[ERROR] Could not open directory %s", path);
        abort();
    }

    char** arr = malloc(sizeof(char*) * _count_files(path));
    int64_t i = 0;
    while ((ent = readdir(dir)) != NULL) {
        if (ent->d_type == DT_REG) {
            arr[i] = strdup(ent->d_name);  // copy the name string
            i++;
        }
    }

    closedir(dir);
    return arr;
}
#endif
//the need for these two functions is yet more evidence of a TERRIBLE FFI design. Whatever shmuck designed this should rebuild it.
int64_t toy_fs_get_file_count_in_dir(ToyPtr path){
    return _count_files((char*) path);
}
int64_t toy_fs_get_folder_count_in_dir(ToyPtr path){
    return _count_dirs((char*) path);
}
//takes a path as a string and will return a char***
ToyArr* toy_fs_read_dir(ToyPtr path){
    int64_t file_count = _count_files((char*) path);
    int64_t folder_count = _count_dirs((char*) path);
    char** files = _list_files((char*) path);
    char** dirs = _list_dirs((char*) path);
    ToyArr* toy_files = (ToyArr*) toy_malloc_arr(file_count, 4, 1);
    ToyArr* toy_folders = (ToyArr*) toy_malloc_arr(folder_count, 4, 1);
    toy_files->should_free_subelements = true;
    toy_folders->should_free_subelements = true;
    for(int i = 0; i < file_count; i++){
        char* t = META_MALLOC(sizeof(files[i]) + 1);
        strcpy(t, files[i]);
        toy_write_to_arr((ToyPtr) toy_files, (int64_t) t, i, 0);
    }
    for(int i = 0; i < folder_count; i++){
        char* t = META_MALLOC(sizeof(files[i]) + 1);
        strcpy(t, files[i]);
        toy_write_to_arr((ToyPtr) toy_folders, (int64_t) t, i, 0);
    }
    ToyArr* arr = (ToyArr*) toy_malloc_arr(2, 4, 2);
    arr->should_free_subelements = true;
    toy_write_to_arr((ToyPtr) arr, (ToyPtr) toy_files, 0, 4);
    toy_write_to_arr((ToyPtr) arr, (ToyPtr) toy_folders, 1, 4);
    return arr;
}