#[cfg(feature = "debug")]
#[macro_export]
macro_rules! debug {
    (targets: [$($target:expr),*], $($arg:tt)*) => {{
        if let Ok(filter) = std::env::var("DEBUG_TARGET") {
            let args = format!("{:?}", ($($arg)*));
            $(
                if filter.contains($target) {
                    dbg!(args.clone());
                }
            )*
        }
    }};
}
#[cfg(not(feature = "debug"))]
#[macro_export]
macro_rules! debug {
    (targets: [$($target:expr),*], $($arg:tt)*) => {};
}
