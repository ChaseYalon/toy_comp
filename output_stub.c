#include <stdint.h>

extern int64_t user_main();


int main(){
    int res = (int) user_main();
    return res;
}