#pragma once
#include <stdint.h>

#define TABLE_SIZE 10

typedef struct _Entry {
    int64_t key;
    int64_t value;
    struct _Entry *next;
} _Entry;

typedef struct {
    _Entry *buckets[TABLE_SIZE];
} ToyHashMap;

void toy_put(int64_t i_map, int64_t key, int64_t value);
int64_t toy_get(int64_t i_map, int64_t key);
int64_t toy_create_map();