//ths file makes he linker happy, found it on a random forum, don ask me what it does
#include <stdlib.h>

typedef struct LLVMOpaqueValue *LLVMValueRef;

LLVMValueRef LLVMConstMul(LLVMValueRef LHS, LLVMValueRef RHS) {
    return (LLVMValueRef)0;
}

LLVMValueRef LLVMConstNSWMul(LLVMValueRef LHS, LLVMValueRef RHS) {
    return (LLVMValueRef)0;
}

LLVMValueRef LLVMConstNUWMul(LLVMValueRef LHS, LLVMValueRef RHS) {
    return (LLVMValueRef)0;
}
