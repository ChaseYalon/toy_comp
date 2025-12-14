#include <stddef.h>
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <math.h>
#include <inttypes.h>

#include "builtins.h"
#include "ctla/ctla.h"


DebugHeap* DEBUG_HEAP = NULL;
__attribute__((constructor))
static void _toy_init() {
    DEBUG_HEAP = DebugHeap_create();
}

//datatype is 0 for string, 1 for bool, 2 for int, 3 for float, 4 for str[], 5 for bool[], 6 for int[], 7 for float[]
//if datatype is 0 (input is string) then nput is a pointer
char* _toy_format(int64_t input, int64_t datatype, int64_t degree) {
    switch(datatype) {
        case 0: { // string
            if (input == 0) {
                const char* literal = "NULL_STRING";
                size_t len = strlen(literal);
                char* buff = malloc(len + 1);
                strcpy(buff, literal);
                return buff;
            } else {
                const char* str = (const char*)input;
                size_t len = strlen(str);
                char* buff = malloc(len + 1);
                strcpy(buff, str);
                return buff;
            }
        }
        case 1: { // boolean
            const char* literal;
            if (input == 1) literal = "true";
            else if (input == 0) literal = "false";
            else {
                fprintf(stderr, "[ERROR] Expected boolean but value was %" PRId64 "\n", input);
                abort();
            }
            size_t len = strlen(literal);
            char* buff = malloc(len + 1);
            strcpy(buff, literal);
            return buff;
        }
        case 2: { // int
            char* buff = malloc(21); // max 64-bit signed int
            sprintf(buff, "%" PRId64, input);
            return buff;
        }
        case 3: { // double
            union { int64_t i; double d; } u;
            u.i = input;
            char* buff = malloc(64);
            snprintf(buff, 64, "%f", u.d);
            return buff;
        }
        case 4: case 5: case 6: case 7: { // arrays
            if (input == 0) {
                const char* literal = "NULL_ARRAY";
                char* buff = malloc(strlen(literal)+1);
                strcpy(buff, literal);
                return buff;
            }

            if (degree <= 0) {
                // just indicate array exists; don’t try to access elements
                const char* literal = "[...]";
                char* buff = malloc(strlen(literal)+1);
                strcpy(buff, literal);
                return buff;
            }

            ToyArr* array = (ToyArr*) input;
            int64_t total_len = 2; // '[' and ']'
            char** element_strs = malloc(sizeof(char*) * array->length);

            for (int64_t i = 0; i < array->length; i++) {
                element_strs[i] = _toy_format(array->arr[i].value, array->arr[i].type, degree - 1);
                total_len += strlen(element_strs[i]);
                if (i != array->length - 1) total_len += 2; // ", "
            }

            char* buff = malloc(total_len + 1);
            char* ptr = buff;
            *ptr++ = '[';
            for (int64_t i = 0; i < array->length; i++) {
                size_t len = strlen(element_strs[i]);
                memcpy(ptr, element_strs[i], len);
                ptr += len;
                if (i != array->length - 1) {
                    *ptr++ = ',';
                    *ptr++ = ' ';
                }
                free(element_strs[i]);
            }
            *ptr++ = ']';
            *ptr = '\0';
            free(element_strs);
            return buff;
        }
        default:
            fprintf(stderr, "[ERROR] Unknown datatype: %" PRId64 "\n", datatype);
            abort();
    }
}


void toy_print(int64_t input, int64_t datatype, int64_t degree) {
    char* buff = (char*) _toy_format(input, datatype, degree);
    printf("%s", buff);
    free(buff);
}

void toy_println(int64_t input, int64_t datatype, int64_t degree) {
    char* buff = (char*) _toy_format(input, datatype, degree);
    printf("%s\n", buff);
    free(buff);
}


//Takes a string at value ptr and puts it in memory, returning its address
int64_t toy_malloc(int64_t ptr) {
    if (ptr == 0){
        fprintf(stderr, "[ERROR] Toy malloc received a null pointer\n");
        abort();
    }
    char* input = (char *)ptr;
    
    size_t len = strlen(input);
    char* out = META_MALLOC(len + 1); //+1 for null terminator
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
    
    char* out = META_MALLOC(combinedLen);
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
        //temporary buffer, does not need META_MALLOC
        char* str = malloc(21);
        if (!str) {
            fprintf(stderr, "[ERROR] Memory allocation failed\n");
            abort();
        }

        sprintf(str, "%"PRId64"", val);
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
        strcpy(out,  buffer);

        return (int64_t)out;
    }

    fprintf(stderr, "[ERROR] Can only convert strings, bools and ints to strings, got type %"PRId64"\n", type);
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
    fprintf(stderr, "[ERROR] Tried to convert type %"PRId64" to bool, that is not supported\n", type);
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
    fprintf(stderr, "[ERROR] Type %"PRId64" unsupported for conversion to int\n", type);
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
        fprintf(stderr, "[ERROR] Attempted to cast non boolean, as boolean, to float (%"PRId64")\n", val);
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
    fprintf(stderr, "[ERROR] Passed unsupported type %"PRId64"\n", type);
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
int64_t toy_malloc_arr(int64_t len, int64_t type) {
    size_t size = (size_t)(len * 16 * 1.4); // allocate 40% more space
    ToyArrVal* arr_ptr = META_MALLOC(size);

    ToyArrVal empty = { .value = 0, .type = 2, ._pad = {0} };

    for (int64_t i = 0; i < len; i++) {
        memcpy((uint8_t*)arr_ptr + i * 16, &empty, 16);
    }

    ToyArr* arr = META_MALLOC(sizeof(ToyArr));
    arr->length = len;
    arr->capacity = (int64_t)(len * 1.4);
    arr->arr = arr_ptr;
    arr->type = type;

    return (int64_t)arr;
}

//type refers to the type of the array NOT the type of its elements or the value being written
void toy_write_to_arr(int64_t arr_in_ptr, int64_t value, int64_t idx, int64_t type) {
    ToyArr* arr_ptr = (ToyArr*) arr_in_ptr;
    if (arr_ptr == NULL){
        fprintf(stderr, "[ERROR] toy_write_to_arr received a null pointer");
        abort();
    }
    if (idx < 0) {
        fprintf(stderr, "[ERROR] Index must be bellow 0, got %"PRId64"", idx);
        abort();
    }
    if (arr_ptr->type != type) {
        fprintf(stderr, "[ERROR] Was expecting type of %"PRId64", got %"PRId64"", arr_ptr->type ,type);
        abort();
    }
    if (idx >= arr_ptr->capacity){
        int64_t new_capacity = (int64_t)(arr_ptr->capacity * 1.4);
        if (idx >= new_capacity) {
            new_capacity = idx * 1.4;
        }

        ToyArrVal* new_data = META_MALLOC(new_capacity * sizeof(ToyArrVal));
        if (!new_data) {
            fprintf(stderr, "[ERROR] Failed to allocate new array buffer\n");
            abort();
        }
        // Copy old arr into new arr
        memcpy(new_data, arr_ptr->arr, arr_ptr->length * sizeof(ToyArrVal));

        // Free old arr - check if this is ok, might cause double free
        free(arr_ptr->arr);

        // Update metadata
        arr_ptr->arr = new_data;
        arr_ptr->capacity = new_capacity;
        arr_ptr->length = idx;
    }
    //If we get here everything is good so we can write the value to the array
    ToyArrVal* elem_ptr = arr_ptr->arr + idx;
    elem_ptr->value = value;
    elem_ptr->type = type;
}

int64_t toy_read_from_arr(int64_t arr_in_ptr, int64_t idx) {
    ToyArr* arr_ptr = (ToyArr*) arr_in_ptr;
    if (arr_ptr == NULL) {
        fprintf(stderr, "[ERROR] toy_read_from_arr got a null pointer");
        abort();
    }
    if (idx > arr_ptr->length) {
        fprintf(stderr, "[ERROR] Tried to read from index %"PRId64" but array is only %"PRId64" elements long", idx, arr_ptr->length);
        abort();
    }
    ToyArrVal* elem = arr_ptr->arr + idx;
    return (int64_t) elem->value; 

}

int64_t toy_arrlen(int64_t arr_in_ptr) {
    ToyArr* arr_ptr = (ToyArr*) arr_in_ptr;
    return arr_ptr->length;
}

char* _read_line() {
    size_t size = 256;
    size_t len = 0;
    //toy_input is marked as allocating a buffer, this is the actual buffer allocation
    char *buffer = META_MALLOC(size);
    if (!buffer) {
        fprintf(stderr, "[ERROR] META_MALLOC failed\n");
        abort();
    }

    for (;;) {
        int c = fgetc(stdin);
        if (c == EOF) {
            if (len == 0) { 
                free(buffer);
                return NULL;
            }
            break;
        }

        if (c == '\n') {
            break;
        }

        buffer[len++] = (char)c;

        // expand buffer if needed
        if (len + 1 >= size) {
            size *= 2;
            char *newbuf = realloc(buffer, size);
            if (!newbuf) {
                free(buffer);
                return NULL;
            }
            buffer = newbuf;
        }
    }

    buffer[len] = '\0';
    return buffer;
}
int64_t toy_input(int64_t i_prompt){
    char* prompt = (char*) i_prompt;
    printf("%s", prompt);
    char* u_in = _read_line();
    return (int64_t) u_in;
}