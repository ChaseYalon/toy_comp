#pragma once
#include <stdint.h>

typedef struct {
    int64_t value;
    uint8_t type;
    uint8_t _pad[7];   //make it align to 16 bytes
} ToyArrVal;
typedef struct {
    int64_t length;
    int64_t capacity;
    int64_t type;
    ToyArrVal* arr;
} ToyArr;

void toy_print(int64_t input, int64_t datatype, int64_t degree);
void toy_println(int64_t input, int64_t datatype, int64_t degree);
int64_t toy_malloc(int64_t ptr);
int64_t toy_concat(int64_t sp1, int64_t sp2);
int64_t toy_strequal(int64_t sp1, int64_t sp2);
int64_t toy_strlen(int64_t sp1);
int64_t toy_type_to_str(int64_t val, int64_t type);
int64_t toy_type_to_bool(int64_t val, int64_t type);
int64_t toy_type_to_int(int64_t val, int64_t type);
int64_t toy_type_to_float(int64_t val, int64_t type);
double toy_int_to_float(int64_t i);
double toy_float_bits_to_double(int64_t f_bits);
int64_t toy_double_to_float_bits(double d);
int64_t toy_malloc_arr(int64_t len, int64_t type);
void toy_write_to_arr(int64_t arr_in_ptr, int64_t value, int64_t idx, int64_t type);
int64_t toy_read_from_arr(int64_t arr_in_ptr, int64_t idx);
int64_t toy_arrlen(int64_t arr_in_ptr);