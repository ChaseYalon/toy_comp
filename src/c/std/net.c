#include "../builtins.h"
#ifdef _WIN32
#include "../../../lib/x86_64-pc-windows-gnu/curl_include/curl.h"
#else
#include "../../../lib/x86_64-unknown-linux-gnu/curl_include/curl.h"
#endif
#include <string.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct {
    char *data;
    size_t len; // bytes currently in data (not including null terminator)
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

    // Reasonable defaults for a runtime
    curl_easy_setopt(curl, CURLOPT_FOLLOWLOCATION, 1L);
    curl_easy_setopt(curl, CURLOPT_TIMEOUT, 30L);
    curl_easy_setopt(curl, CURLOPT_CONNECTTIMEOUT, 10L);

    curlRes = curl_easy_perform(curl);
    if (curlRes != CURLE_OK) {
        fprintf(stderr, "[ERROR] curl_easy_perform() failed: %s\n",
                curl_easy_strerror(curlRes));
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