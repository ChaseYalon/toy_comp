#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <math.h>

//datatype is 0 for string, 1 for bool, 2 for int, 3 for float
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
    if (datatype == 3) {
        union { int64_t i; double d; } u;
        u.i = input;
        printf("%f", u.d);
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
    if (datatype == 3) {
        union { int64_t i; double d; } u;
        u.i = input;
        printf("%f\n", u.d);
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
        fprintf(stderr, "[ERROR] Tried to convert non boolean to string as bool\n");
        abort();
    }
    if (type == 2) {
        char* str = malloc(21);
        if (!str) {
            fprintf(stderr, "[ERROR] Memory allocation failed\n");
            abort();
        }

        sprintf(str, "%lld", (long long)val);
        int64_t out = toy_malloc((int64_t)str);
        free(str); //not actual value, temp buffer
        return out;
    }
    if (type == 3) {
        union { int64_t i; double d; } u;
        u.i = val;

        char buffer[64];
        snprintf(buffer, sizeof(buffer), "%g", u.d);

        char* out = (char*) toy_malloc(strlen(buffer) + 1);
        strcpy_s(out, strlen(out) + 1, buffer);

        return (int64_t)out;
    }

    fprintf(stderr, "[ERROR] Can only convert strings, bools and ints to strings, got type %lld\n", type);
    abort();
}

int64_t toy_type_to_bool(int64_t val, int64_t type) {
    if (type == 0){
        char* t = "true";
        char* f = "false";
        if ( toy_strequal(val, (int64_t) t) ) {
            return 1;
        }
        if ( toy_strequal(val, (int64_t) f)) {
            return 0;
        }
        fprintf(stderr, "[ERROR] tried to convert string to bool that was not \"true\" or \"false\"\n");
        abort();
    }
    if (type == 1) {
        return val;
    }
    if (type == 2) {
        if (val == 1) {
            return 1;
        }
        if (val == 0) {
            return 0;
        }
        fprintf(stderr, "[ERROR] Tried to convert int (that was not 1 or 0) to bool\n");
        abort();
    }
    if (type == 3) {
        union { int64_t i; double d; } u;
        u.i = val;
        return (u.d < 0.0) ? 0 : 1; //negative false, positive true

    }
    fprintf(stderr, "[ERROR] Tried to convert type %lld to bool, that is not supported\n", type);
    abort();
}

int64_t toy_type_to_int(int64_t val, int64_t type) {
    if (type == 0){
        char* str = (char*) val;
        if (str == NULL){
            fprintf(stderr, "[ERROR] toy_type_to_int received a null pointer");
            abort();
        }
        errno = 0;
        char* endptr;
        int64_t endval = strtoll(str, &endptr, 10);

        if (errno != 0) {
            perror("[ERROR] strtoll failed");
            abort();
        }
        if (*endptr != '\0') {
            fprintf(stderr, "[ERROR] String contains non-numeric characters: '%s'\n", str);
            abort();
        }
        return (int64_t) endval;
    }
    if (type == 1) {
        if (val == 1){
            return 1;
        }
        if (val == 0){
            return 0;
        }
        fprintf(stderr, "[ERROR] Tried to convert boolean to integer but input was not boolean\n");
        abort();
    }
    if (type == 2){
        return val;
    }
    if (type == 3){
        union { int64_t i; double d; } u;
        u.i = val;

        double rounded = round(u.d); 

        return (int64_t) rounded;
    }
    fprintf(stderr, "[ERROR] Type %lld unsupported for conversion to int\n", type);
    abort();
}

int64_t toy_type_to_float(int64_t val, int64_t type) {
    if (type == 0) {
        char* str = (char*) val;

        errno = 0;
        char* endptr;
        double d = strtod(str, &endptr);

        if (errno != 0) {
            perror("[ERROR] strtod failed");
            abort();
        }
        if (*endptr != '\0') {
            fprintf(stderr, "[ERROR] String contains non-numeric characters: '%s'\n", str);
            abort();
        }

        union { double d; int64_t i; } u;
        u.d = d;
        return u.i;
    }
    if (type == 1) {
        if (val == 0) {
            return 0.0;
        }
        if (val == 1){
            return 1.0;
        }
        fprintf(stderr, "[ERROR] Attempted to cast non boolean, as boolean, to float (%llx)\n", val);
        abort();
    }
    if (type == 2) {
        union { double d; int64_t i; } u;
        u.d = (double)val;
        return u.i;      
    }
    if (type == 3) {
        return val;
    }
    fprintf(stderr, "[ERROR] Passed unsupported type %lld\n", type);
    abort();
}

double toy_int_to_float(int64_t i) {
    return (double)i;
}

double toy_float_bits_to_double(int64_t f_bits) {
    union { int64_t i; double d; } u;
    u.i = f_bits;
    return u.d;
}

int64_t toy_double_to_float_bits(double d) {
    union { int64_t i; double d; } u;
    u.d = d;
    return u.i;
}