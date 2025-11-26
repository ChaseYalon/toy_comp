#include "hashmap.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>


int64_t _hash(void *key, int64_t capacity) {
    return ((int64_t)key >> 3) % capacity;
}

DebugMap* DebugMap_create() {
    DebugMap *map = malloc(sizeof(DebugMap));
    map->capacity = INITIAL_CAPACITY;
    map->size = 0;
    map->buckets = calloc(map->capacity, sizeof(Entry*));
    return map;
}

void DebugMap_put(DebugMap *map, void *key, int64_t value) {
    size_t idx = _hash(key, map->capacity);
    Entry *entry = map->buckets[idx];
    
    // Update existing key
    while (entry) {
        if (entry->key == key) {
            entry->value = value;
            return;
        }
        entry = entry->next;
    }
    
    // Insert new entry
    Entry *new_entry = malloc(sizeof(Entry));
    new_entry->key = key;
    new_entry->value = value;
    new_entry->next = map->buckets[idx];
    map->buckets[idx] = new_entry;
    map->size++;
}

int DebugMap_get(DebugMap *map, void *key, int64_t *value) {
    size_t idx = _hash(key, map->capacity);
    Entry *entry = map->buckets[idx];
    
    while (entry) {
        if (entry->key == key) {
            *value = entry->value;
            return 1;
        }
        entry = entry->next;
    }
    return 0;
}

void DebugMap_free(DebugMap *map) {
    for (int64_t i = 0; i < map->capacity; i++) {
        Entry *entry = map->buckets[i];
        while (entry) {
            Entry *tmp = entry;
            entry = entry->next;
            free(tmp);
        }
    }
    free(map->buckets);
    free(map);

}