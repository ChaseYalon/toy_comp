#include <stdio.h>
#include <stdint.h>

extern int64_t user_main();


int main(){
    int res = (int) user_main();
    printf("User main returned %d\n", res);
    return res;
}