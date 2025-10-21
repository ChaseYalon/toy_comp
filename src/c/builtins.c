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
    strcpy_s(out, len + 1, input);
    
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
    
    strcpy_s(out, combinedLen, str1);
    strcat_s(out,combinedLen, str2);
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

int64_t toy_strlen(int64_t sp1) {
    if (sp1 == 0) {
        fprintf(stderr, "[ERROR] toy_strlen received a null pointer\n");
        abort();
    }
    char* str1 = (char*) sp1;
    return strlen(str1);
}

//val is the value and t is the input type, 0 for str, 1 for bool, 2 for int
int64_t toy_type_to_str(int64_t val, int64_t type) {
    if (type == 0) {
        //It is a string, return the value without changing it
        return val;
    }
    if (type == 1) {
        if (val == 0) {
            char* str = "false";
            return toy_malloc((int64_t)str);
        }
        if (val == 1) {
            char* str = "true";
            return toy_malloc((int64_t) str);
        }
        fprintf(stderr, "[ERROR] Tried to convert non boolean to string as bool");
        abort();
    }
    if (type == 2) {
        int64_t tmp = val;
        int len = (tmp <= 0) ? 1 : 0; // negative or zero
        int64_t t = tmp;
        while (t != 0) {
            t /= 10;
            len++;
        }

        char* str = malloc(len + 1); // +1 for null terminator
        if (!str) {
            fprintf(stderr, "[ERROR] Memory allocation failed\n");
            abort();
        }

        sprintf(str, "%lld", (long long)val);
        int64_t out = toy_malloc((int64_t)str);
        free(str); //not actual value, temp buffer
        return out;
    }
    fprintf(stderr, "[ERROR] Can only convert strings, bools and ints to strings, got type %lld", type);
    abort();
}