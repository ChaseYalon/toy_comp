use crate::ToyPtr;
use std::env;
use std::sync::atomic::{AtomicU64, Ordering};

/// Iterations a generated loop is allowed at process start.
const MAX_ITERS: f64 = 5.0;
/// Backing storage for the decay rate. Written exactly once, by [`init_fuzz_half_life`].
static HALF_LIFE_MS: AtomicU64 = AtomicU64::new(1000);
/// Reads `TOY_FUZZ_HALF_LIFE_MS` from the environment. Called once from `stub::init` before any toy
/// code runs; nothing writes this again for the rest of the process.
pub fn init_fuzz_half_life() {
    if let Some(ms) = env::var("TOY_FUZZ_HALF_LIFE_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
    {
        HALF_LIFE_MS.store(ms, Ordering::Relaxed);
    }
}
/// How many iterations a fuzzer-generated loop may run right now.
///
/// Decays exponentially with process age: 5 iterations at startup, halving every `HALF_LIFE_MS`,
/// floored at 0. A floor of 0 still runs each loop body ONCE, because the generated guard sits at
/// the end of the body — so a long-running program keeps executing its full structure (every call,
/// every free, a meaningful leak check at exit) and only stops paying for repetition.
///
/// Deliberately global rather than per-loop or per-function: a budget handed out per scope
/// multiplies exactly like the iteration counts it is meant to bound, because every loop entry and
/// every invocation starts a fresh one.
///
/// Returns a toy `int`, which this backend represents as i64.
#[unsafe(no_mangle)]
fn toy_fuzz_loop_budget() -> ToyPtr {
    let half_life = HALF_LIFE_MS.load(Ordering::Relaxed) as f64;
    let elapsed = crate::stub::program_elapsed_ms() as f64;
    return (MAX_ITERS * 0.5_f64.powf(elapsed / half_life)).floor() as ToyPtr;
}
