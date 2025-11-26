#pragma once
#include <stdint.h>

#define INITIAL_CAPACITY 16
#define LOAD_FACTOR 0.75

typedef struct Entry {
    void *key;
    int64_t value;
    struct Entry *next;
} Entry;

typedef struct {
    Entry **buckets;
    int64_t capacity;
    int64_t size;
} DebugMap;

int64_t _hash(void *key, int64_t capacity);
DebugMap* DebugMap_create();
void DebugMap_put(DebugMap *map, void *key, int64_t value);
int DebugMap_get(DebugMap *map, void *key, int64_t *value);
void DebugMap_free(DebugMap *map);
void _PrintDebug_map(DebugMap* map);