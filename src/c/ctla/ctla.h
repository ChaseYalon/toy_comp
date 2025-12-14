#pragma once
#include <stdint.h>
#include "hashmap.h"

typedef struct {
    DebugMap* Map;
    int64_t TotalLiveAllocations;
} DebugHeap;

DebugHeap* DebugHeap_create();
void DebugHeap_free(DebugHeap*d);
void* ToyMallocDebug(size_t size, DebugHeap* d);
void toy_free(void* buff);
void _PrintDebug_heap(DebugHeap* d);