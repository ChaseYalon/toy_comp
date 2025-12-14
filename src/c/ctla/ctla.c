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
    if(strcmp(getenv("TOY_DEBUG"), "TRUE") == 0) {

        DebugMap_put(DEBUG_HEAP->Map, buff, -1);
        DEBUG_HEAP->TotalLiveAllocations--;
    }
    free(buff);
}

void _PrintDebug_heap(DebugHeap* d) {
    _PrintDebug_map(d->Map);
    printf("Total Live entries remaining: %lld\n", d->TotalLiveAllocations);

}