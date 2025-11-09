#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>
#include "hashmap.h"

//random hash function from the internet
unsigned int _hash(int64_t i_key) {
    const unsigned char* key = (const unsigned char*) i_key; 
    unsigned long hash = 5381;
    int c;

    while ((c = *key++))
        hash = ((hash << 5) + hash) + c;  // hash * 33 + c

    return (unsigned int)(hash % TABLE_SIZE);
}

_Entry *create_Entry(int64_t i_key, int64_t value) {
    _Entry *_Entry = malloc(sizeof(*_Entry));
    _Entry->key = i_key;
    _Entry->value = value;
    _Entry->next = NULL;
    return _Entry;
}

void toy_put(int64_t i_map, int64_t key, int64_t value) {
    ToyHashMap* map = (ToyHashMap*) i_map;
    unsigned int index = _hash(key);
    _Entry *current = map->buckets[index];

    while (current != NULL) {
        if (current->key == key) {
            current->value = value; // Update existing
            return;
        }
        current = current->next;
    }

    // Insert new _Entry at head of list
    _Entry *new_Entry = create_Entry(key, value);
    new_Entry->next = map->buckets[index];
    map->buckets[index] = new_Entry;
}

int64_t toy_get(int64_t i_map, int64_t key) {
    ToyHashMap* map = (ToyHashMap*) i_map;
    unsigned int index = _hash(key);
    _Entry *current = map->buckets[index];

    while (current != NULL) {
        printf("DEBUG: hash(%lld) = %u\n", key, _hash(key));
        if (current->key == key)
            return current->value;
        current = current->next;
    }
    fprintf(stderr, "[ERROR] Item not found in ToyHashMap, if you are seeing this, something has gone very wrong in the compiler, this error should be impossible");
    abort();
    return -1; // Not found
}
int64_t toy_create_map() {
    ToyHashMap *m = malloc(sizeof(ToyHashMap));
    for (int i = 0; i < TABLE_SIZE; i++) {
        m->buckets[i] = NULL;
    }
    return (int64_t)m;
}
