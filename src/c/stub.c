//this function exists to make openssl happy, hope it doesnot break something :D
int atexit(void (*func)(void)) {
    (void)func;
    return 0;
}

#include "builtins.h"
#include "ctla/ctla.h"
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

#ifdef _WIN32
  #include <windows.h>
#else
  #include <unistd.h>
  #include <fcntl.h>
#endif

extern int64_t user_main();
int64_t GLOBAL_ARGC;
char** GLOBAL_ARGV;

CURL* curl = NULL;
CURLcode curlRes = CURLE_OK;

// objcopy'd bundle symbols (matches your nm output exactly)
#ifdef _WIN32
    extern const unsigned char _binary_lib_x86_64_pc_windows_gnu_cacert_pem_start[];
    extern const unsigned char _binary_lib_x86_64_pc_windows_gnu_cacert_pem_end[];
    #define _cert_start _binary_lib_x86_64_pc_windows_gnu_cacert_pem_start
    #define _cert_end _binary_lib_x86_64_pc_windows_gnu_cacert_pem_end
#else
    extern const unsigned char _binary_cacert_pem_start[];
    extern const unsigned char _binary_cacert_pem_end[];
    #define _cert_start _binary_cacert_pem_start
    #define _cert_end _binary_cacert_pem_end
#endif
static char g_ca_temp_path[1024];

void _SetDebug_env() {
#ifdef _WIN32
    _putenv_s("TOY_DEBUG", "TRUE");
#endif
#ifdef __linux__
    setenv("TOY_DEBUG", "TRUE", 1);
#endif
}

static int write_bytes_to_temp_file(const void *data, size_t len, char *out_path, size_t out_path_cap) {
#ifdef _WIN32
    char tmp_dir[MAX_PATH];
    DWORD n = GetTempPathA((DWORD)sizeof(tmp_dir), tmp_dir);
    if (n == 0 || n >= sizeof(tmp_dir)) {
        fprintf(stderr, "[ERROR] GetTempPathA failed\n");
        return 0;
    }

    char tmp_file[MAX_PATH];
    if (GetTempFileNameA(tmp_dir, "toy", 0, tmp_file) == 0) {
        fprintf(stderr, "[ERROR] GetTempFileNameA failed\n");
        return 0;
    }

    FILE *f = fopen(tmp_file, "wb");
    if (!f) {
        fprintf(stderr, "[ERROR] fopen(temp) failed\n");
        DeleteFileA(tmp_file);
        return 0;
    }

    if (len != 0 && fwrite(data, 1, len, f) != len) {
        fprintf(stderr, "[ERROR] fwrite(temp) failed\n");
        fclose(f);
        DeleteFileA(tmp_file);
        return 0;
    }
    fclose(f);

    if (strlen(tmp_file) + 1 > out_path_cap) {
        fprintf(stderr, "[ERROR] temp path too long\n");
        DeleteFileA(tmp_file);
        return 0;
    }
    strcpy(out_path, tmp_file);
    return 1;
#else
    char base_tmpl[] = "/tmp/toy-cacert-XXXXXX";
    int fd = mkstemp(base_tmpl);
    if (fd < 0) {
        perror("[ERROR] mkstemp");
        return 0;
    }

    const unsigned char *p = (const unsigned char*)data;
    size_t off = 0;
    while (off < len) {
        ssize_t w = write(fd, p + off, len - off);
        if (w <= 0) {
            perror("[ERROR] write(temp)");
            close(fd);
            unlink(base_tmpl);
            return 0;
        }
        off += (size_t)w;
    }
    close(fd);

    if (strlen(base_tmpl) + 1 > out_path_cap) {
        fprintf(stderr, "[ERROR] temp path too long\n");
        unlink(base_tmpl);
        return 0;
    }
    strcpy(out_path, base_tmpl);
    return 1;
#endif
}

static void remove_temp_file(const char *path) {
    if (!path || !path[0]) return;
#ifdef _WIN32
    DeleteFileA(path);
#else
    unlink(path);
#endif
}

static void toy_curl_init_with_embedded_ca(void) {
    if (curl) return;

    curl_global_init(CURL_GLOBAL_DEFAULT);
    curl = curl_easy_init();
    if (!curl) {
        fprintf(stderr, "[ERROR] curl_easy_init failed\n");
        abort();
    }

    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 1L);
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYHOST, 2L);
    curl_easy_setopt(curl, CURLOPT_FOLLOWLOCATION, 1L);
    const unsigned char *start = _cert_start;
    const unsigned char *end   = _cert_end;
    size_t ca_len = (size_t)(end - start);

    g_ca_temp_path[0] = '\0';
    if (!write_bytes_to_temp_file(start, ca_len, g_ca_temp_path, sizeof(g_ca_temp_path))) {
        fprintf(stderr, "[ERROR] failed to write embedded CA bundle to temp file\n");
        abort();
    }

    curl_easy_setopt(curl, CURLOPT_CAINFO, g_ca_temp_path);
}

int main(int argc, char** argv) {
    toy_curl_init_with_embedded_ca();

    GLOBAL_ARGC = (int64_t)argc;
    GLOBAL_ARGV = malloc(sizeof(char*) * argc);
    if (!GLOBAL_ARGV) {
        fprintf(stderr, "Failed to allocate GLOBAL_ARGV\n");
        abort();
    }

    for (int i = 0; i < argc; i++) {
        size_t len = strlen(argv[i]) + 1;
        GLOBAL_ARGV[i] = malloc(len);
        if (!GLOBAL_ARGV[i]) {
            fprintf(stderr, "Failed to allocate argv string\n");
            abort();
        }
        memcpy(GLOBAL_ARGV[i], argv[i], len);
    }

    _SetDebug_env();
    int res = (int)user_main();

    for (int i = 0; i < argc; i++) {
        free(GLOBAL_ARGV[i]);
    }
    free(GLOBAL_ARGV);
    GLOBAL_ARGV = NULL;

    if (getenv("TOY_DEBUG") != NULL && strcmp(getenv("TOY_DEBUG"), "TRUE") == 0 &&
        DEBUG_HEAP->TotalLiveAllocations != 0) {
        _PrintDebug_heap(DEBUG_HEAP);
        printf("\nFAIL_TEST\n");
    }

    if (curl) {
        curl_easy_cleanup(curl);
        curl = NULL;
    }
    curl_global_cleanup();

    remove_temp_file(g_ca_temp_path);
    g_ca_temp_path[0] = '\0';

    return res;
}