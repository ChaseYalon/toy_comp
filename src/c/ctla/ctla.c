#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include "ctla.h"
#include "hashmap.h"
#include "../builtins.h"
DebugHeap* DebugHeap_create() {
    DebugMap* m = DebugMap_create();
    DebugHeap* d = malloc(sizeof(DebugHeap));
    d->Map = m;
    d->TotalLiveAllocations = 0;
    return d;
}

void DebugHeap_free(DebugHeap*d) {
    DebugMap_free(d->Map);
    if (d->TotalLiveAllocations != 0) {
        fprintf(stderr, "[WARN] There are %lld live allocations remaining, at heap deallocations\n", d->TotalLiveAllocations);
    }
    free(d);

}

void* ToyMallocDebug(size_t size, DebugHeap* d) {
    void* buff = malloc(size);
    DebugMap_put(d->Map, buff, size);
    d->TotalLiveAllocations++;
    return buff;
}
void toy_free(void* buff) {
    if (!buff){
        fprintf(stderr, "[ERROR] Tried to free a null buffer\n");
        abort();
    }
    if(getenv("TOY_DEBUG") && strcmp(getenv("TOY_DEBUG"), "TRUE") == 0) {
        // Only decrement if this pointer was actually tracked (has a non-negative value)
        int64_t value;
        if (DebugMap_get(DEBUG_HEAP->Map, buff, &value) && value >= 0) {
            DEBUG_HEAP->TotalLiveAllocations--;
        }
        DebugMap_put(DEBUG_HEAP->Map, buff, -1);
    }
    free(buff);
}
void _PrintDebug_heap(DebugHeap* d) {
    _PrintDebug_map(d->Map);
    printf("Total Live entries remaining: %lld\n", d->TotalLiveAllocations);

}

void _CheckUseAfterFree(void* buff) {
    if (!buff) {
        return; // NULL pointers are handled separately
    }
    if (getenv("TOY_DEBUG") != NULL && strcmp(getenv("TOY_DEBUG"), "TRUE") == 0) {
        int64_t value;
        if (DebugMap_get(DEBUG_HEAP->Map, buff, &value) && value == -1) {
            fprintf(stderr, "[ERROR] Use-after-free detected! Pointer %p was already freed\n", buff);
            printf("\nFAIL_TEST\n");
            fflush(stdout);
            fflush(stderr);
            abort();
        }
    }
}