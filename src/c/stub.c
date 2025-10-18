#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

extern int64_t user_main();

void toy_print(char* input) {
    printf("%s", input);
}

void toy_println(char* input) {
    printf("%s\n", input);
}

//Takes a string at value ptr and puts it in memory, returning its address
int64_t toy_malloc(int64_t* ptr) {
    if (ptr == NULL){
        fprintf(stderr, "[ERROR] Toy malloc received a null pointer\n");
        abort();
    }
    char *input = (char *)ptr;

    char* out = malloc(strlen(input) + 1); //+1 for null terminator (fuck c)
    if (out == NULL){
        fprintf(stderr, "[ERROR] Toy malloc failed\n");
        abort();
    }
    strcpy_s(out, strlen(input) + 1, input);

    return (int64_t) out;
}


int main(){
    int res = (int) user_main();
    printf("User main returned %d\n", res);
    return res;
}