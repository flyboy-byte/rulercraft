// SPDX-License-Identifier: MIT OR Apache-2.0
//! Spreading independent per-item work over the machine's cores with the standard library only.
//!
//! Only the per-item work runs in parallel; results come back in item order and any sum over them is
//! taken by the caller, in that order, on one thread. So every result is bit-for-bit the same for any
//! thread count, including one.
//!
//! The thread count is the machine's available parallelism, or `RULERCRAFT_THREADS` when set to a whole
//! number above 0, or what [`with_threads`] sets for the current thread.
use std::cell::Cell;

/// Fewest items worth a thread of their own.
const MIN_PER_THREAD: usize = 512;

thread_local! {
    static OVERRIDE: Cell<Option<usize>> = const { Cell::new(None) };
}

/// The number of threads parallel work started from this thread will use (at least 1).
pub fn threads() -> usize {
    if let Some(n) = OVERRIDE.with(Cell::get) {
        return n;
    }
    std::env::var("RULERCRAFT_THREADS")
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from))
}

/// Runs `f` with parallel work started from this thread limited to `n` threads (0 counts as 1). The
/// earlier setting comes back afterwards, even if `f` panics.
pub fn with_threads<R>(n: usize, f: impl FnOnce() -> R) -> R {
    struct Restore(Option<usize>);
    impl Drop for Restore {
        fn drop(&mut self) {
            OVERRIDE.with(|c| c.set(self.0));
        }
    }
    let _restore = Restore(OVERRIDE.with(|c| c.replace(Some(n.max(1)))));
    f()
}

/// `f(0), f(1), ..., f(n - 1)` in order, computed on up to [`threads`] threads in contiguous chunks.
/// A panic in `f` is passed on to the caller.
pub fn map<R: Send>(n: usize, f: impl Fn(usize) -> R + Sync) -> Vec<R> {
    let k = threads().min(n / MIN_PER_THREAD).max(1);
    if k == 1 {
        return (0..n).map(f).collect();
    }
    let chunk = n.div_ceil(k);
    let f = &f;
    std::thread::scope(|s| {
        let workers: Vec<_> = (0..k)
            .map(|c| {
                s.spawn(move || {
                    (c * chunk..((c + 1) * chunk).min(n))
                        .map(f)
                        .collect::<Vec<R>>()
                })
            })
            .collect();
        let mut out = Vec::with_capacity(n);
        for w in workers {
            match w.join() {
                Ok(part) => out.extend(part),
                Err(e) => std::panic::resume_unwind(e),
            }
        }
        out
    })
}
