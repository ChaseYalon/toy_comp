use chrono::{Datelike, Local};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
#[unsafe(no_mangle)]
fn toy_time_ms_since_unix_epoch() -> i64 {
    let now = SystemTime::now();
    let since_the_epoch = now.duration_since(UNIX_EPOCH).unwrap(); //unsafe    
    let in_ms = since_the_epoch.as_millis();
    return in_ms as i64;
}
#[unsafe(no_mangle)]
fn toy_time_current_year() -> i64 {
    return Local::now().year() as i64;
}
#[unsafe(no_mangle)]
fn toy_time_current_month() -> i64 {
    let now = Local::now();
    return now.month() as i64;
}
#[unsafe(no_mangle)]
fn toy_time_current_day() -> i64 {
    return Local::now().day() as i64;
}
#[unsafe(no_mangle)]
fn toy_time_sleep(ms: i64) {
    thread::sleep(Duration::from_millis(ms as u64));
}
