#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

//datatype is 0 for string, 1 for bool, 2 for int
//if datatype is 0 (input is string) then nput is a pointer
void toy_print(int64_t input, int64_t datatype) {
    if (datatype == 0) {
        if (input == 0) {
            return;
        }
        printf("%s", (char*)input);
        return;
    }
    if (datatype == 1) {
        if (input == 1) {
            printf("true");
            return;
        }
        if (input == 0) {
            printf("false");
            return;
        }
        fprintf(stderr, "[ERROR] Expected boolean but value was %lld\n", input);
        return;
    }
    if (datatype == 2) {
        // Int - input is the actual value
        printf("%lld", input);
        return;
    }
    fprintf(stderr, "[ERROR] Unknown datatype of %lld\n", datatype);
}

void toy_println(int64_t input, int64_t datatype) {
    if (datatype == 0) {
        if (input == 0) {
            printf("\n");
            return;
        }
        printf("%s\n", (char*)input);
        return;
    }
    if (datatype == 1) {
        if (input == 1) {
            printf("true\n");
            return;
        }
        if (input == 0) {
            printf("false\n");
            return;
        }
        fprintf(stderr, "[ERROR] Expected boolean but value was %lld\n", input);
        return;
    }
    if (datatype == 2) {
        printf("%lld\n", input);
        return;
    }
    fprintf(stderr, "[ERROR] Unknown datatype of %lld\n", datatype);
}


//Takes a string at value ptr and puts it in memory, returning its address
int64_t toy_malloc(int64_t ptr) {
    if (ptr == 0){
        fprintf(stderr, "[ERROR] Toy malloc received a null pointer\n");
        abort();
    }
    char* input = (char *)ptr;
    
    size_t len = strlen(input);
    char* out = malloc(len + 1); //+1 for null terminator
    if (out == NULL){
        fprintf(stderr, "[ERROR] Toy malloc failed\n");
        abort();
    }
    strcpy(out, input);
    
    return (int64_t) out;
}

//Concats two strings together, returning a third string
int64_t toy_concat(int64_t sp1, int64_t sp2) {
    if (sp1 == 0 || sp2 == 0){
        fprintf(stderr, "[ERROR] Toy concat received a null pointer\n");
        abort();
    }
    char* str1 = (char *) sp1;
    char* str2 = (char *) sp2;
    
    size_t len1 = strlen(str1);
    size_t len2 = strlen(str2);
    size_t combinedLen = len1 + len2 + 1; // +1 for null terminator
    
    char* out = malloc(combinedLen);
    if (out == NULL){
        fprintf(stderr, "[ERROR] Malloc failed\n");
        abort();
    }
    
    strcpy(out, str1);
    strcat(out, str2);
    return (int64_t) out;
}

int64_t toy_strequal(int64_t sp1, int64_t sp2) {
    char* str1 = (char*) sp1;
    char* str2 = (char*) sp2;
    
    if (strcmp(str1, str2) == 0) {
        return 1; //boolean true
    } else {
        return 0; //boolean false
    }
}