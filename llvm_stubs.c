//at some point this should be moved to lib, but it is a pain to rezip and reupload to source forge (see setup_build_system.py)
typedef struct LLVMOpaqueValue *LLVMValueRef;

LLVMValueRef LLVMConstMul(LLVMValueRef LHS, LLVMValueRef RHS) {
    (void)LHS;
    (void)RHS;
    return (LLVMValueRef)0;
}

LLVMValueRef LLVMConstNSWMul(LLVMValueRef LHS, LLVMValueRef RHS) {
    (void)LHS;
    (void)RHS;
    return (LLVMValueRef)0;
}

LLVMValueRef LLVMConstNUWMul(LLVMValueRef LHS, LLVMValueRef RHS) {
    (void)LHS;
    (void)RHS;
    return (LLVMValueRef)0;
}