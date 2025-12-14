#include "builtins.h"
#include "ctla/ctla.h"
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
extern int64_t user_main();

//sets environment variables, everything is in debug mode by default
void _SetDebug_env() {
    #ifdef _WIN32
        _putenv_s("TOY_DEBUG", "TRUE");
    #endif
    #ifdef __LINUX__
        setenv("TOY_DEBUG", "TRUE", 1);
    #endif
}

int main(){
    //_SetDebug_env();
    int res = (int) user_main();
    //if it is greater then 0 there is a memory leak, if it is less then 0 it is a double free, still need to detect 
    //use after free
    if (getenv("TOY_DEBUG")!= NULL && strcmp(getenv("TOY_DEBUG"), "TRUE") == 0 && DEBUG_HEAP->TotalLiveAllocations != 0){
        _PrintDebug_heap(DEBUG_HEAP);
        //if this env is set, in a test and this is the signal to fail it
        printf("\nFAIL_TEST\n");
    }
    return res;
}