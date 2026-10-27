//! Minimal data parallelism over `std::thread::scope` (no extra crates).

use std::sync::OnceLock;

/// Number of worker threads (available parallelism, at least 1).
pub fn threads() -> usize {
    static N: OnceLock<usize> = OnceLock::new();
    *N.get_or_init(|| std::thread::available_parallelism().map_or(1, |n| n.get()))
}

/// `(0..n).map(f).collect()`, split across threads in contiguous chunks
/// when `n ≥ min_n`. Output order is the index order, independent of
/// scheduling.
pub fn par_map_min<T: Send>(n: usize, min_n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let t = threads();
    if n < min_n.max(2) || t == 1 {
        return (0..n).map(f).collect();
    }
    let chunk = n.div_ceil(t);
    let f = &f;
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..n)
            .step_by(chunk)
            .map(|start| {
                let end = (start + chunk).min(n);
                s.spawn(move || (start..end).map(f).collect::<Vec<T>>())
            })
            .collect();
        let mut out = Vec::with_capacity(n);
        for h in handles {
            out.extend(h.join().expect("worker panicked"));
        }
        out
    })
}

/// `par_map_min` with a threshold for cheap per-item work.
pub fn par_map<T: Send>(n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    par_map_min(n, 1024, f)
}
