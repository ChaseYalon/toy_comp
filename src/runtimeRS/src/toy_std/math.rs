// trig
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_sin(value: i64) -> i64 {
    ((value as f64).sin() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_sinf(value: f64) -> f64 {
    value.sin()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_cos(value: i64) -> i64 {
    ((value as f64).cos() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_cosf(value: f64) -> f64 {
    value.cos()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_tan(value: i64) -> i64 {
    ((value as f64).tan() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_tanf(value: f64) -> f64 {
    value.tan()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_asin(value: i64) -> i64 {
    ((value as f64).asin() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_asinf(value: f64) -> f64 {
    value.asin()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_acos(value: i64) -> i64 {
    ((value as f64).acos() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_acosf(value: f64) -> f64 {
    value.acos()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_atan(value: i64) -> i64 {
    ((value as f64).atan() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_atanf(value: f64) -> f64 {
    value.atan()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_atan2(y: i64, x: i64) -> i64 {
    ((y as f64).atan2(x as f64) * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_atan2f(y: f64, x: f64) -> f64 {
    y.atan2(x)
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_abs(value: i64) -> i64 {
    value.abs()
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_absf(value: f64) -> f64 {
    value.abs()
}

// hyperbolic
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_sinh(value: i64) -> i64 {
    ((value as f64).sinh() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_sinhf(value: f64) -> f64 {
    value.sinh()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_cosh(value: i64) -> i64 {
    ((value as f64).cosh() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_coshf(value: f64) -> f64 {
    value.cosh()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_tanh(value: i64) -> i64 {
    ((value as f64).tanh() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_tanhf(value: f64) -> f64 {
    value.tanh()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_asinh(value: i64) -> i64 {
    ((value as f64).asinh() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_asinhf(value: f64) -> f64 {
    value.asinh()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_acosh(value: i64) -> i64 {
    ((value as f64).acosh() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_acoshf(value: f64) -> f64 {
    value.acosh()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_atanh(value: i64) -> i64 {
    ((value as f64).atanh() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_atanhf(value: f64) -> f64 {
    value.atanh()
}

// exponential and logarithmic
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_exp(value: i64) -> i64 {
    ((value as f64).exp() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_expf(value: f64) -> f64 {
    value.exp()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_exp2(value: i64) -> i64 {
    ((value as f64).exp2() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_exp2f(value: f64) -> f64 {
    value.exp2()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_expm1(value: i64) -> i64 {
    ((value as f64).exp_m1() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_expm1f(value: f64) -> f64 {
    value.exp_m1()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_log(value: i64) -> i64 {
    ((value as f64).ln() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_logf(value: f64) -> f64 {
    value.ln()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_log10(value: i64) -> i64 {
    ((value as f64).log10() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_log10f(value: f64) -> f64 {
    value.log10()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_log2(value: i64) -> i64 {
    ((value as f64).log2() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_log2f(value: f64) -> f64 {
    value.log2()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_log1p(value: i64) -> i64 {
    ((value as f64).ln_1p() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_log1pf(value: f64) -> f64 {
    value.ln_1p()
}

// Power functions
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_pow(base: i64, exp: i64) -> i64 {
    ((base as f64).powf(exp as f64) * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_powf(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_sqrt(value: i64) -> i64 {
    ((value as f64).sqrt() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_sqrtf(value: f64) -> f64 {
    value.sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_cbrt(value: i64) -> i64 {
    ((value as f64).cbrt() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_cbrtf(value: f64) -> f64 {
    value.cbrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_hypot(x: i64, y: i64) -> i64 {
    ((x as f64).hypot(y as f64) * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_hypotf(x: f64, y: f64) -> f64 {
    x.hypot(y)
}

// Nearest integer operations
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_ceil(value: i64) -> i64 {
    value
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_ceilf(value: f64) -> f64 {
    value.ceil()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_floor(value: i64) -> i64 {
    value
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_floorf(value: f64) -> f64 {
    value.floor()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_trunc(value: i64) -> i64 {
    value
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_truncf(value: f64) -> f64 {
    value.trunc()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_round(value: i64) -> i64 {
    value
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_roundf(value: f64) -> f64 {
    value.round()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_nearbyint(value: i64) -> i64 {
    value
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_nearbyintf(value: f64) -> f64 {
    value.round()
}

// Remainder functions
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_fmod(x: i64, y: i64) -> i64 {
    ((x as f64) % (y as f64) * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_fmodf(x: f64, y: f64) -> f64 {
    x % y
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_remainder(x: i64, y: i64) -> i64 {
    let xf = x as f64;
    let yf = y as f64;
    ((xf - (xf / yf).round() * yf) * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_remainderf(x: f64, y: f64) -> f64 {
    x - (x / y).round() * y
}

// Manipulation functions
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_ldexp(x: i64, exp: i64) -> i64 {
    ((x as f64) * (2.0f64).powi(exp as i32)) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_ldexpf(x: f64, exp: i64) -> f64 {
    x * (2.0f64).powi(exp as i32)
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_scalbn(x: i64, n: i64) -> i64 {
    ((x as f64) * (2.0f64).powi(n as i32)) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_scalbnf(x: f64, n: i64) -> f64 {
    x * (2.0f64).powi(n as i32)
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_scalbln(x: i64, n: i64) -> i64 {
    ((x as f64) * (2.0f64).powi(n as i32)) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_scalblnf(x: f64, n: i64) -> f64 {
    x * (2.0f64).powi(n as i32)
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_ilogb(x: i64) -> i64 {
    (x as f64).log2().trunc() as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_ilogbf(x: f64) -> i64 {
    x.log2().trunc() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_logb(x: i64) -> i64 {
    ((x as f64).log2().trunc() * 1e6) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_logbf(x: f64) -> f64 {
    x.log2().trunc()
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_copysign(x: i64, y: i64) -> i64 {
    (x as f64).copysign(y as f64) as i64
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_copysignf(x: f64, y: f64) -> f64 {
    x.copysign(y)
}

// Difference, min, max
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_dim(x: i64, y: i64) -> i64 {
    if x > y { x - y } else { 0 }
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_dimf(x: f64, y: f64) -> f64 {
    (x - y).max(0.0)
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_max(x: i64, y: i64) -> i64 {
    x.max(y)
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_maxf(x: f64, y: f64) -> f64 {
    x.max(y)
}

#[unsafe(no_mangle)]
pub extern "C" fn toy_math_min(x: i64, y: i64) -> i64 {
    x.min(y)
}
#[unsafe(no_mangle)]
pub extern "C" fn toy_math_minf(x: f64, y: f64) -> f64 {
    x.min(y)
}
