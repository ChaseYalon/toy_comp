#include <math.h>
#include <stdint.h>
#include <stdlib.h>
//trig
int64_t toy_math_sin(int64_t value) {
    return (int64_t)(sin((double)value) * 1e6);
}
double toy_math_sinf(double value) {
    return sin(value);
}

int64_t toy_math_cos(int64_t value) {
    return (int64_t)(cos((double)value) * 1e6);
}
double toy_math_cosf(double value) {
    return cos(value);
}

int64_t toy_math_tan(int64_t value) {
    return (int64_t)(tan((double)value) * 1e6);
}
double toy_math_tanf(double value) {
    return tan(value);
}
int64_t toy_math_asin(int64_t value) {
    return (int64_t)(asin((double)value) * 1e6);
}
double toy_math_asinf(double value) {
    return asin(value);
}
int64_t toy_math_acos(int64_t value) {
    return (int64_t)(acos((double)value) * 1e6);
}
double toy_math_acosf(double value) {
    return acos(value);
}
int64_t toy_math_atan(int64_t value) {
    return (int64_t)(atan((double)value) * 1e6);
}
double toy_math_atanf(double value) {
    return atan(value);
}
int64_t toy_math_atan2(int64_t y, int64_t x) {
    return (int64_t)(atan2((double)y, (double)x) * 1e6);
}
int64_t toy_math_atan2f(double y, double x) {
    return atan2(y, x);
}

int64_t toy_math_abs(int64_t value) {
    return llabs(value);
}
double toy_math_absf(double value) {
    return fabs(value);
}

//hyperbolic
int64_t toy_math_sinh(int64_t value) {
    return (int64_t)(sinh((double)value) * 1e6);
}
double toy_math_sinhf(double value) {
    return sinh(value);
}

int64_t toy_math_cosh(int64_t value) {
    return (int64_t)(cosh((double)value) * 1e6);
}
double toy_math_coshf(double value) {
    return cosh(value);
}

int64_t toy_math_tanh(int64_t value) {
    return (int64_t)(tanh((double)value) * 1e6);
}
double toy_math_tanhf(double value) {
    return tanh(value);
}

int64_t toy_math_asinh(int64_t value) {
    return (int64_t)(asinh((double)value) * 1e6);
}
double toy_math_asinhf(double value) {
    return asinh(value);
}

int64_t toy_math_acosh(int64_t value) {
    return (int64_t)(acosh((double)value) * 1e6);
}
double toy_math_acoshf(double value) {
    return acosh(value);
}

int64_t toy_math_atanh(int64_t value) {
    return (int64_t)(atanh((double)value) * 1e6);
}
double toy_math_atanhf(double value) {
    return atanh(value);
}

//exponential and logarithmic
int64_t toy_math_exp(int64_t value) {
    return (int64_t)(exp((double)value) * 1e6);
}
double toy_math_expf(double value) {
    return exp(value);
}

int64_t toy_math_exp2(int64_t value) {
    return (int64_t)(exp2((double)value) * 1e6);
}
double toy_math_exp2f(double value) {
    return exp2(value);
}

int64_t toy_math_expm1(int64_t value) {
    return (int64_t)(expm1((double)value) * 1e6);
}
double toy_math_expm1f(double value) {
    return expm1(value);
}

int64_t toy_math_log(int64_t value) {
    return (int64_t)(log((double)value) * 1e6);
}
double toy_math_logf(double value) {
    return log(value);
}
int64_t toy_math_log10(int64_t value) {
    return (int64_t)(log10((double)value) * 1e6);
}
double toy_math_log10f(double value) {
    return log10(value);
}

int64_t toy_math_log2(int64_t value) {
    return (int64_t)(log2((double)value) * 1e6);
}
double toy_math_log2f(double value) {
    return log2(value);
}

int64_t toy_math_log1p(int64_t value) {
    return (int64_t)(log1p((double)value) * 1e6);
}
double toy_math_log1pf(double value) {
    return log1p(value);
}

// Power functions
int64_t toy_math_pow(int64_t base, int64_t exp) {
    return (int64_t)(pow((double)base, (double)exp) * 1e6);
}
double toy_math_powf(double base, double exp) {
    return pow(base, exp);
}

int64_t toy_math_sqrt(int64_t value) {
    return (int64_t)(sqrt((double)value) * 1e6);
}
double toy_math_sqrtf(double value) {
    return sqrt(value);
}

int64_t toy_math_cbrt(int64_t value) {
    return (int64_t)(cbrt((double)value) * 1e6);
}
double toy_math_cbrtf(double value) {
    return cbrt(value);
}

int64_t toy_math_hypot(int64_t x, int64_t y) {
    return (int64_t)(hypot((double)x, (double)y) * 1e6);
}
double toy_math_hypotf(double x, double y) {
    return hypot(x, y);
}

// Nearest integer operations
int64_t toy_math_ceil(int64_t value) {
    return value;
}
double toy_math_ceilf(double value) {
    return ceil(value);
}

int64_t toy_math_floor(int64_t value) {
    return value;
}
double toy_math_floorf(double value) {
    return floor(value);
}

int64_t toy_math_trunc(int64_t value) {
    return value;
}
double toy_math_truncf(double value) {
    return trunc(value);
}

int64_t toy_math_round(int64_t value) {
    return value;
}
double toy_math_roundf(double value) {
    return round(value);
}

int64_t toy_math_nearbyint(int64_t value) {
    return value;
}
double toy_math_nearbyintf(double value) {
    return nearbyint(value);
}

// Remainder functions
int64_t toy_math_fmod(int64_t x, int64_t y) {
    return (int64_t)(fmod((double)x, (double)y) * 1e6);
}
double toy_math_fmodf(double x, double y) {
    return fmod(x, y);
}

int64_t toy_math_remainder(int64_t x, int64_t y) {
    return (int64_t)(remainder((double)x, (double)y) * 1e6);
}
double toy_math_remainderf(double x, double y) {
    return remainder(x, y);
}

// Manipulation functions
int64_t toy_math_ldexp(int64_t x, int64_t exp) {
    return (int64_t)(ldexp((double)x, (int)exp));
}
double toy_math_ldexpf(double x, int64_t exp) {
    return ldexp(x, (int)exp);
}

int64_t toy_math_scalbn(int64_t x, int64_t n) {
    return (int64_t)(scalbn((double)x, (int)n));
}
double toy_math_scalbnf(double x, int64_t n) {
    return scalbn(x, (int)n);
}

int64_t toy_math_scalbln(int64_t x, int64_t n) {
    return (int64_t)(scalbln((double)x, (long)n));
}
double toy_math_scalblnf(double x, int64_t n) {
    return scalbln(x, (long)n);
}

int64_t toy_math_ilogb(int64_t x) {
    return (int64_t)ilogb((double)x);
}
int64_t toy_math_ilogbf(double x) {
    return (int64_t)ilogb(x);
}

int64_t toy_math_logb(int64_t x) {
    return (int64_t)(logb((double)x) * 1e6);
}
double toy_math_logbf(double x) {
    return logb(x);
}

int64_t toy_math_nextafter(int64_t x, int64_t y) {
    return (int64_t)(nextafter((double)x, (double)y) * 1e6);
}
double toy_math_nextafterf(double x, double y) {
    return nextafter(x, y);
}

int64_t toy_math_nexttoward(int64_t x, int64_t y) {
    return (int64_t)(nexttoward((double)x, (long double)y) * 1e6);
}
double toy_math_nexttowardf(double x, double y) {
    return nexttoward(x, (long double)y);
}

int64_t toy_math_copysign(int64_t x, int64_t y) {
    return (int64_t)copysign((double)x, (double)y);
}
double toy_math_copysignf(double x, double y) {
    return copysign(x, y);
}

// Difference, min, max
int64_t toy_math_dim(int64_t x, int64_t y) {
    return x > y ? x - y : 0;
}
double toy_math_dimf(double x, double y) {
    return fdim(x, y);
}

int64_t toy_math_max(int64_t x, int64_t y) {
    return x > y ? x : y;
}
double toy_math_maxf(double x, double y) {
    return fmax(x, y);
}

int64_t toy_math_min(int64_t x, int64_t y) {
    return x < y ? x : y;
}
double toy_math_minf(double x, double y) {
    return fmin(x, y);
}