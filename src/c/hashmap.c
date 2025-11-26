#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include "hashmap.h"
#include "builtins.h"

int64_t toy_create_map() {
    ToyHashMap *m = META_MALLOC(sizeof(ToyHashMap));
    m->count = 0;
    for (int i = 0; i < MAX_ENTRIES; i++)
        m->entries[i] = NULL;
    return (int64_t)m;
}

void toy_put(int64_t i_map, int64_t key, int64_t value) {
    ToyHashMap *map = (ToyHashMap*) i_map;

    // Update if key exists
    for (int i = 0; i < map->count; i++) {
        if (strcmp((char*)map->entries[i]->key, (char*) key) == 0) {
            map->entries[i]->value = value;
            return;
        }
    }

    // Add new entry
    if (map->count >= MAX_ENTRIES) {
        fprintf(stderr, "ERROR: exceeded max entries\n");
        abort();
    }

    _Entry *e = META_MALLOC(sizeof(_Entry));
    e->key = key;
    e->value = value;
    map->entries[map->count++] = e;
}

void print_map(ToyHashMap* m) {
    for (int i = 0; i < MAX_ENTRIES; i++) {
        printf("{KEY: %s, ", (char*) m->entries[i]->key);
        printf("VALUE: %lld}\n", m->entries[i]->value);
    }
}

int64_t toy_get(int64_t i_map, int64_t key) {
    ToyHashMap *map = (ToyHashMap*) i_map;

    for (int i = 0; i < map->count; i++) {
        if (strcmp((char*) map->entries[i]->key, (char*)key) == 0)
            return map->entries[i]->value;
    }
    print_map(map);
    fprintf(stderr, "[ERROR] key not found in ToyHashMap\n");
    abort();
    return -1;
}
