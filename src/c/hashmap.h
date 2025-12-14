#pragma once
#include <stdint.h>

#define MAX_ENTRIES 300 //if you have more then 300 keys in a struct, you have fucked something up

typedef struct _Entry {
    int64_t key;
    int64_t value;
} _Entry;

typedef struct {
    _Entry *entries[MAX_ENTRIES];
    int count;
} ToyHashMap;

void toy_put(int64_t i_map, int64_t key, int64_t value);
int64_t toy_get(int64_t i_map, int64_t key);
int64_t toy_create_map();