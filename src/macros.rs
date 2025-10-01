///Global debug macro that runs if feature debug is enabled
#[cfg(feature = "debug")]
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        dbg!($($arg)*);
    };
}
///Ignore all debug macros b/c feature flag is disabled
#[cfg(not(feature = "debug"))]
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {}; // does nothing
}
