#include "../builtins.h"
#include <psdk_inc/_socket_types.h>
#include <stdint.h>

#ifdef _WIN32
  #include <winsock2.h>
  #include <ws2tcpip.h>
  #pragma comment(lib, "Ws2_32.lib")
#else
  #include <errno.h>
  #include <unistd.h>
  #include <sys/types.h>
  #include <sys/socket.h>
  #include <netinet/in.h>
  #include <arpa/inet.h>
#endif

#ifdef _WIN32
#include "../../../lib/x86_64-pc-windows-gnu/curl_include/curl.h"
#else
#include "../../../lib/x86_64-unknown-linux-gnu/curl_include/curl.h"
#endif

#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>

typedef struct {
    char *data;
    size_t len;
} CurlBuf;

static size_t _write_to_buff(void *contents, size_t size, size_t nmemb, void *userp) {
    size_t chunk_len = size * nmemb;
    CurlBuf *buf = (CurlBuf *)userp;

    char *new_data = (char *)META_MALLOC(buf->len + chunk_len + 1);
    if (!new_data) return 0;

    if (buf->data && buf->len > 0) {
        memcpy(new_data, buf->data, buf->len);
        toy_free(buf->data);
    }

    memcpy(new_data + buf->len, contents, chunk_len);
    buf->len += chunk_len;
    new_data[buf->len] = '\0';

    buf->data = new_data;
    return chunk_len;
}

ToyPtr toy_net_get_url(ToyPtr url) {
    if (!curl) {
        fprintf(stderr, "[ERROR] curl is not initialized. Call curl init at startup.\n");
        abort();
    }

    CurlBuf buf = {0};

    curl_easy_setopt(curl, CURLOPT_URL, (const char *)url);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, _write_to_buff);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &buf);

    curl_easy_setopt(curl, CURLOPT_FOLLOWLOCATION, 1L);
    curl_easy_setopt(curl, CURLOPT_TIMEOUT, 30L);
    curl_easy_setopt(curl, CURLOPT_CONNECTTIMEOUT, 10L);

    curlRes = curl_easy_perform(curl);
    if (curlRes != CURLE_OK) {
        fprintf(stderr, "[ERROR] curl_easy_perform() failed: %s\n", curl_easy_strerror(curlRes));
        abort();
    }

    long status = 0;
    curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &status);
    if (status < 200 || status >= 300) {
        fprintf(stderr, "[ERROR] Received status code %ld\n", status);
        abort();
    }

    if (!buf.data) {
        char *empty = (char *)META_MALLOC(1);
        empty[0] = '\0';
        return (ToyPtr)empty;
    }

    return (ToyPtr)buf.data;
}

// demo server config
InternalHttpServerConfig* global_config = NULL;

#ifdef _WIN32
static SOCKET listen_sock = INVALID_SOCKET;
static int wsa_inited = 0;

static void _wsa_init_once(void) {
    if (wsa_inited) return;
    WSADATA wsa;
    int r = WSAStartup(MAKEWORD(2,2), &wsa);
    if (r != 0) {
        fprintf(stderr, "[ERROR] WSAStartup failed: %d\n", r);
        abort();
    }
    wsa_inited = 1;
}

static void _sock_perror(const char* what) {
    int e = WSAGetLastError();
    fprintf(stderr, "[ERROR] %s failed: WSAGetLastError=%d\n", what, e);
}
#else
static int listen_sock = -1;

static void _sock_perror(const char* what) {
    fprintf(stderr, "[ERROR] %s failed: errno=%d (%s)\n", what, errno, strerror(errno));
}
#endif

void toy_net_configure_http_server(int64_t port, int64_t timeout) {
    (void)timeout;

#ifdef _WIN32
    _wsa_init_once();
    if (listen_sock != INVALID_SOCKET) return;

    listen_sock = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (listen_sock == INVALID_SOCKET) {
        _sock_perror("socket");
        abort();
    }

    BOOL opt = TRUE;
    setsockopt(listen_sock, SOL_SOCKET, SO_REUSEADDR, (const char*)&opt, sizeof(opt));

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(INADDR_ANY);
    addr.sin_port = htons((uint16_t)port);

    if (bind(listen_sock, (struct sockaddr*)&addr, sizeof(addr)) == SOCKET_ERROR) {
        _sock_perror("bind");
        abort();
    }

    if (listen(listen_sock, SOMAXCONN) == SOCKET_ERROR) {
        _sock_perror("listen");
        abort();
    }
#else
    if (listen_sock != -1) return;

    listen_sock = socket(AF_INET, SOCK_STREAM, 0);
    if (listen_sock < 0) {
        _sock_perror("socket");
        abort();
    }

    int opt = 1;
    setsockopt(listen_sock, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));

    struct sockaddr_in addr = {0};
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(INADDR_ANY);
    addr.sin_port = htons((uint16_t)port);

    if (bind(listen_sock, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        _sock_perror("bind");
        abort();
    }

    if (listen(listen_sock, SOMAXCONN) < 0) {
        _sock_perror("listen");
        abort();
    }
#endif
}

ToyPtr toy_net_connection_requested() {
#ifdef _WIN32
    if (listen_sock == INVALID_SOCKET) {
        fprintf(stderr, "Server not configured\n");
        abort();
    }

    fd_set fds;
    FD_ZERO(&fds);
    FD_SET(listen_sock, &fds);

    struct timeval tv;
    tv.tv_sec = 0;
    tv.tv_usec = 0;

    int res = select(0, &fds, NULL, NULL, &tv);
    if (res > 0 && FD_ISSET(listen_sock, &fds)) return (ToyPtr)1;
    return (ToyPtr)0;
#else
    if (listen_sock < 0) {
        fprintf(stderr, "Server not configured\n");
        abort();
    }

    fd_set fds;
    FD_ZERO(&fds);
    FD_SET(listen_sock, &fds);

    struct timeval tv;
    tv.tv_sec = 0;
    tv.tv_usec = 0;

    int res = select(listen_sock + 1, &fds, NULL, NULL, &tv);
    if (res > 0 && FD_ISSET(listen_sock, &fds)) return (ToyPtr)1;
    return (ToyPtr)0;
#endif
}

#ifdef _WIN32
  #define STRNICMP _strnicmp
#else
  #include <strings.h>
  #define STRNICMP strncasecmp
#endif

static int find_header_end(const char *buf, int len) {
    if (len < 4) return -1;
    for (int i = 0; i <= len - 4; i++) {
        if (buf[i] == '\r' && buf[i+1] == '\n' && buf[i+2] == '\r' && buf[i+3] == '\n')
            return i + 4;
    }
    return -1;
}

static void parse_request_line(const char *buf, int header_end,
                               const char **method, int *method_len,
                               const char **path, int *path_len,
                               const char **version, int *version_len) {
    int line_end = -1;
    for (int i = 0; i + 1 < header_end; i++) {
        if (buf[i] == '\r' && buf[i+1] == '\n') { line_end = i; break; }
    }
    if (line_end < 0) { fprintf(stderr, "[ERROR] malformed request line\n"); abort(); }

    int p0 = 0;
    int p1 = -1, p2 = -1;

    for (int i = p0; i < line_end; i++) { if (buf[i] == ' ') { p1 = i; break; } }
    if (p1 < 0) { fprintf(stderr, "[ERROR] malformed request line\n"); abort(); }

    for (int i = p1 + 1; i < line_end; i++) { if (buf[i] == ' ') { p2 = i; break; } }
    if (p2 < 0) { fprintf(stderr, "[ERROR] malformed request line\n"); abort(); }

    *method = buf + p0;      *method_len = p1 - p0;
    *path   = buf + p1 + 1;  *path_len   = p2 - (p1 + 1);
    *version= buf + p2 + 1;  *version_len= line_end - (p2 + 1);
}

static int parse_content_length_and_check_chunked(const char *buf, int header_end) {
    int content_length = 0;

    int i = 0;
    while (i + 1 < header_end && !(buf[i] == '\r' && buf[i+1] == '\n')) i++;
    if (i + 1 >= header_end) { fprintf(stderr, "[ERROR] malformed headers\n"); abort(); }
    i += 2;

    while (i < header_end - 2) {
        if (buf[i] == '\r' && buf[i+1] == '\n') break;

        int line_end = -1;
        for (int j = i; j + 1 < header_end; j++) {
            if (buf[j] == '\r' && buf[j+1] == '\n') { line_end = j; break; }
        }
        if (line_end < 0) break;

        const char *te = "Transfer-Encoding:";
        int te_len = (int)strlen(te);
        if (line_end - i >= te_len && STRNICMP(buf + i, te, te_len) == 0) {
            for (int k = i + te_len; k + 7 <= line_end; k++) {
                if (STRNICMP(buf + k, "chunked", 7) == 0) {
                    fprintf(stderr, "[ERROR] chunked requests not supported (prototype)\n");
                    abort();
                }
            }
        }

        const char *cl = "Content-Length:";
        int cl_len = (int)strlen(cl);
        if (line_end - i >= cl_len && STRNICMP(buf + i, cl, cl_len) == 0) {
            int p = i + cl_len;
            while (p < line_end && (buf[p] == ' ' || buf[p] == '\t')) p++;

            long v = 0;
            int any = 0;
            while (p < line_end && buf[p] >= '0' && buf[p] <= '9') {
                any = 1;
                v = v * 10 + (buf[p] - '0');
                p++;
            }
            if (!any || v < 0 || v > 100000000) {
                fprintf(stderr, "[ERROR] invalid Content-Length\n");
                abort();
            }
            content_length = (int)v;
        }

        i = line_end + 2;
    }

    return content_length;
}

static char* dup_slice_as_cstr(const char* p, int n) {
    if (n < 0) n = 0;
    char* out = (char*)META_MALLOC((size_t)n + 1);
    if (!out) { fprintf(stderr, "[ERROR] OOM\n"); abort(); }
    if (n > 0) memcpy(out, p, (size_t)n);
    out[n] = '\0';
    return out;
}
static SOCKET global_socket_bodge = 0;
ToyArr* toy_net_read_request() {
#ifdef _WIN32
    if (listen_sock == INVALID_SOCKET) {
        fprintf(stderr, "[ERROR] HTTP-Unconfigured\n");
        abort();
    }
    struct sockaddr_in client_addr;
    int addr_len = sizeof(client_addr);
    SOCKET client = accept(listen_sock, (struct sockaddr*)&client_addr, &addr_len);
    global_socket_bodge = client;
    if (client == INVALID_SOCKET) {
        _sock_perror("accept");
        abort();
    }
#else
    if (listen_sock < 0) {
        fprintf(stderr, "[ERROR] HTTP-Unconfigured\n");
        abort();
    }
    struct sockaddr_in client_addr;
    socklen_t addr_len = sizeof(client_addr);
    int client = accept(listen_sock, (struct sockaddr*)&client_addr, &addr_len);
    if (client < 0) {
        _sock_perror("accept");
        abort();
    }
#endif

    char inputBuffer[8192];
    int total = 0;
    int header_end = -1;

    while (header_end < 0) {
        if (total >= (int)sizeof(inputBuffer)) {
            fprintf(stderr, "[ERROR] headers too large\n");
            goto fail_close;
        }

        int n = recv(client, inputBuffer + total, (int)sizeof(inputBuffer) - total, 0);
        if (n == 0) {
            fprintf(stderr, "[ERROR] client closed before sending headers\n");
            goto fail_close;
        }
        if (n < 0) {
            _sock_perror("recv");
            goto fail_close;
        }

        total += n;
        header_end = find_header_end(inputBuffer, total);
    }

    const char *method = NULL, *path = NULL, *version = NULL;
    int method_len = 0, path_len = 0, version_len = 0;
    parse_request_line(inputBuffer, header_end, &method, &method_len, &path, &path_len, &version, &version_len);

    int content_length = parse_content_length_and_check_chunked(inputBuffer, header_end);

    int body_have = total - header_end;
    while (body_have < content_length) {
        if (total >= (int)sizeof(inputBuffer)) {
            fprintf(stderr, "[ERROR] request too large for prototype buffer\n");
            goto fail_close;
        }

        int n = recv(client, inputBuffer + total, (int)sizeof(inputBuffer) - total, 0);
        if (n == 0) {
            fprintf(stderr, "[ERROR] client closed before full body\n");
            goto fail_close;
        }
        if (n < 0) {
            _sock_perror("recv");
            goto fail_close;
        }

        total += n;
        body_have = total - header_end;
    }

    const char *body = inputBuffer + header_end;
    int body_len = content_length;

    char* method_s = dup_slice_as_cstr(method, method_len);
    char* path_s   = dup_slice_as_cstr(path, path_len);
    char* body_s   = dup_slice_as_cstr(body, body_len);

    ToyArr* arr = (ToyArr*)toy_malloc_arr(4, 0, 1);
    char* client_str = META_MALLOC(64);
    snprintf(client_str, 64, "%llu", client);
#ifdef _WIN32
    toy_write_to_arr((ToyPtr)arr, (ToyPtr)client_str, 0, 0);
#else
    toy_write_to_arr((ToyPtr)arr, (ToyPtr)(int64_t)client, 0, 0);
#endif
    toy_write_to_arr((ToyPtr)arr, (ToyPtr)method_s, 1, 0);
    toy_write_to_arr((ToyPtr)arr, (ToyPtr)path_s,   2, 0);
    toy_write_to_arr((ToyPtr)arr, (ToyPtr)body_s,   3, 0);

    return arr;

fail_close:
#ifdef _WIN32
    closesocket(client);
#else
    close(client);
#endif
    return NULL;
}

void toy_net_close_client() {
#ifdef _WIN32
    if ((SOCKET)global_socket_bodge != INVALID_SOCKET) closesocket((SOCKET)global_socket_bodge);
#else
    if ((int)global_socket_bodge >= 0) close((int)global_socket_bodge);
#endif
}

static int _send_all_client(int64_t client_handle, const char* data, int len) {
#ifdef _WIN32
    SOCKET s = (SOCKET)client_handle;
    int sent = 0;
    while (sent < len) {
        int n = send(s, data + sent, len - sent, 0);
        if (n == SOCKET_ERROR) {
            _sock_perror("send");
            return -1;
        }
        if (n == 0) return -1;
        sent += n;
    }
    return 0;
#else
    int s = (int)client_handle;
    int sent = 0;
    while (sent < len) {
        int n = (int)send(s, data + sent, (size_t)(len - sent), 0);
        if (n < 0) {
            _sock_perror("send");
            return -1;
        }
        if (n == 0) return -1;
        sent += n;
    }
    return 0;
#endif
}

static const char* _http_reason_phrase(int status) {
    switch (status) {
        case 200: return "OK";
        case 201: return "Created";
        case 204: return "No Content";
        case 301: return "Moved Permanently";
        case 302: return "Found";
        case 400: return "Bad Request";
        case 401: return "Unauthorized";
        case 403: return "Forbidden";
        case 404: return "Not Found";
        case 405: return "Method Not Allowed";
        case 413: return "Payload Too Large";
        case 500: return "Internal Server Error";
        case 502: return "Bad Gateway";
        case 503: return "Service Unavailable";
        default:  return "OK";
    }
}

void toy_net_write_response(int64_t status_code, ToyPtr content_type, ToyPtr body) {
    const char* ct = content_type ? (const char*)content_type : "text/plain; charset=utf-8";
    const char* b  = body ? (const char*)body : "";
    int status = (int)status_code;

    // If your Toy strings can contain NUL bytes, you need a length parameter instead of strlen().
    int body_len = (int)strlen(b);

    char header[1024];
    int header_len = snprintf(
        header, sizeof(header),
        "HTTP/1.1 %d %s\r\n"
        "Content-Type: %s\r\n"
        "Content-Length: %d\r\n"
        "Connection: close\r\n"
        "\r\n",
        status, _http_reason_phrase(status),
        ct,
        body_len
    );

    if (header_len < 0 || header_len >= (int)sizeof(header)) {
        fprintf(stderr, "[ERROR] response headers too large\n");
        return;
    }

    if (_send_all_client(global_socket_bodge, header, header_len) != 0) return;
    if (body_len > 0) {
        if (_send_all_client(global_socket_bodge, b, body_len) != 0) return;
    }
}