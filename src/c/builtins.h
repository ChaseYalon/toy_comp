#pragma once
#define CURL_STATICLIB
#ifdef _WIN32
#include "../../lib/x86_64-pc-windows-gnu/curl_include/curl.h"
#else
#include "../../lib/x86_64-unknown-linux-gnu/curl_include/curl.h"
#endif
#include <stdint.h>
#include "ctla/ctla.h"

extern DebugHeap* DEBUG_HEAP;
extern int64_t GLOBAL_ARGC;
extern char** GLOBAL_ARGV;
#define META_MALLOC(size) \
    ((getenv("TOY_DEBUG") && strcmp(getenv("TOY_DEBUG"), "TRUE") == 0) \
        ? ToyMallocDebug(size, DEBUG_HEAP) \
        : malloc(size))

typedef struct {
    int64_t value;
    uint8_t type;
    uint8_t _pad[7];   //make it align to 16 bytes
} ToyArrVal;
typedef struct {
    int64_t length;
    int64_t capacity;
    int64_t type;
    int64_t degree;
    ToyArrVal* arr;
} ToyArr;
typedef int64_t ToyPtr;
void toy_print(ToyPtr input, int64_t datatype, int64_t degree);
void toy_println(ToyPtr input, int64_t datatype, int64_t degree);
ToyPtr toy_malloc(ToyPtr ptr);
ToyPtr toy_concat(ToyPtr sp1, ToyPtr sp2);
int64_t toy_strequal(ToyPtr sp1, ToyPtr sp2);
int64_t toy_strlen(ToyPtr sp1);
//val could be a ToyPtr if it s a string
ToyPtr toy_type_to_str(int64_t val, int64_t type);
//val could be a ToyPtr if it s a string
int64_t toy_type_to_bool(int64_t val, int64_t type);
//val could be a ToyPtr if it s a string
int64_t toy_type_to_int(int64_t val, int64_t type);
//val could be a ToyPtr if it s a string
int64_t toy_type_to_float(int64_t val, int64_t type);
double toy_int_to_float(int64_t i);
double toy_float_bits_to_double(int64_t f_bits);
int64_t toy_double_to_float_bits(double d);
ToyPtr toy_malloc_struct(int64_t size, ToyPtr toy_struct);
ToyPtr toy_malloc_arr(int64_t len, int64_t type, int64_t degree);
void toy_write_to_arr(ToyPtr arr_in_ptr, int64_t value, int64_t idx, int64_t type);
//return value could be a pointer if sizeof(elem_type) > wordSize
int64_t toy_read_from_arr(ToyPtr arr_in_ptr, int64_t idx);
int64_t toy_arrlen(ToyPtr arr_in_ptr);
ToyPtr toy_input(ToyPtr i_prompt);
void toy_free_arr(ToyPtr arr_ptr_int);

extern CURL* curl;
extern CURLcode curlRes;
void toy_net_init(void);
void toy_net_shutdown(void);
typedef struct{
    int acceptConnectionsOn;
    int maxTimeoutS;
} InternalHttpServerConfig;

extern InternalHttpServerConfig* global_config;