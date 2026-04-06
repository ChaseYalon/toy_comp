#pragma once
#include <stddef.h>
#include <stdint.h>
typedef int64_t ToyPtr;
//DEREFRENCING A DebugHeap in C IS UB B/C IT USES A RUST HASHMAP!!!!!!!!!
//USE should_fail TO DETERMINE WHEN IT NEEDS TO FAIL
typedef struct DebugHeap{
    void* Map; //hash map is opaque to C
    int64_t TotalLiveAllocations;
    int64_t TotalAllocations;
} DebugHeap;
DebugHeap* DebugHeap_create();
void DebugHeap_free(DebugHeap*d);
void* ToyMallocDebug(size_t size, DebugHeap* d);
void toy_free(void* buff);
void _PrintDebug_heap(DebugHeap* d);
void _CheckUseAfterFree(void* buff);
int64_t should_fail();
//datatype is 0 for string, 1 for bool, 2 for int, 3 for float, 4 for str[], 5 for bool[], 6 for int[], 7 for float[], 8 for struct[]
//if datatype is 0 (input is string) then input is a pointer
//Input could be an int, if sizeof(type) > wordSize
char* _toy_format(ToyPtr input, int64_t datatype, int64_t degree);