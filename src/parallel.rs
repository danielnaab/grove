//! Run blocking work (git, graft) in parallel with scoped threads. No async runtime.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const WORKERS: usize = 8;

/// `items.iter().map(f)`, on up to 8 threads, in order.
pub fn map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<R>>> = Mutex::new(items.iter().map(|_| None).collect());
    std::thread::scope(|scope| {
        for _ in 0..WORKERS.min(items.len()) {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(item) = items.get(i) else { break };
                    let result = f(item);
                    results.lock().expect("no worker panics while holding the lock")[i] = Some(result);
                }
            });
        }
    });
    results.into_inner().expect("workers are done").into_iter().map(|r| r.expect("every item ran")).collect()
}
