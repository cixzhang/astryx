//! Threads that paint a frame's bands, kept for the renderer's life.
//!
//! Starting a thread takes the dynamic loader's lock (for its thread-local
//! storage). While another thread loads a large library, such as a GL
//! driver, a frame that started its painting threads waited for that load:
//! 20 to 35 ms. Kept threads start once and then only wait for work.

use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

// SAFETY, for every `unsafe` in this file: `Pool::run` lends its helper
// threads a pointer to the caller's closure, and does not return or unwind
// until every call on that closure has returned. It waits until all calls
// are counted done after making its own, and a call that panics is caught
// and counted, never unwound through `run`. So no thread uses the pointer
// after the closure's borrow ends, and the closure is `Sync`, so calling it
// from several threads at once is allowed.

/// Work for the pool: called once with each index below `count`.
#[derive(Clone, Copy)]
struct Work {
    /// The caller's closure: valid while `Pool::run` runs (see SAFETY).
    f: *const (dyn Fn(usize) + Sync + 'static),
    count: usize,
}

// SAFETY: see the top of the file.
unsafe impl Send for Work {}

#[derive(Default)]
struct State {
    work: Option<Work>,
    /// The next index to hand out.
    next: usize,
    /// Calls that have returned.
    done: usize,
    /// A call panicked: its payload, for the caller.
    panic: Option<Box<dyn std::any::Any + Send>>,
    quit: bool,
}

pub(crate) struct Shared {
    state: Mutex<State>,
    /// Work arrived, or the pool ends.
    wake: Condvar,
    /// The last call of the work returned.
    finished: Condvar,
}

impl Pool {
    /// Gone once the pool and every one of its threads are.
    #[cfg(test)]
    pub(crate) fn watch(&self) -> std::sync::Weak<Shared> {
        Arc::downgrade(&self.shared)
    }
}

/// Painting threads.
pub(crate) struct Pool {
    shared: Arc<Shared>,
    threads: Vec<JoinHandle<()>>,
}

impl Pool {
    /// A pool of `helpers` threads; the thread that runs work also works.
    /// Threads that cannot start are left out.
    pub(crate) fn new(helpers: usize) -> Self {
        Self::named(helpers, "astryx-paint")
    }

    /// As [`Self::new`], its threads named `<name>-<n>`.
    fn named(helpers: usize, name: &str) -> Self {
        let shared = Arc::new(Shared {
            state: Mutex::new(State::default()),
            wake: Condvar::new(),
            finished: Condvar::new(),
        });
        let threads = (0..helpers)
            .filter_map(|i| {
                let shared = shared.clone();
                std::thread::Builder::new()
                    .name(format!("{name}-{i}"))
                    .spawn(move || helper(&shared))
                    .ok()
            })
            .collect();
        Self { shared, threads }
    }

    /// Calls `f` once with each index below `count`, on the pool's threads
    /// and this one, and returns when every call has returned. A panic in a
    /// call panics here.
    pub(crate) fn run(&self, count: usize, f: &(dyn Fn(usize) + Sync)) {
        if count == 0 {
            return;
        }
        // SAFETY: see the top of the file; only the lifetime changes.
        let f: &'static (dyn Fn(usize) + Sync) = unsafe { std::mem::transmute(f) };
        let work = Work { f, count };
        {
            let mut state = self.shared.state.lock().expect("pool state");
            state.work = Some(work);
            state.next = 0;
            state.done = 0;
        }
        self.shared.wake.notify_all();
        let mut state = self.shared.state.lock().expect("pool state");
        // Work alongside the helpers, then wait for theirs.
        while let Some(i) = take(&mut state) {
            drop(state);
            let called = catch_unwind(AssertUnwindSafe(|| f(i)));
            state = self.shared.state.lock().expect("pool state");
            finish(&mut state, called, &self.shared);
        }
        while state.done < count {
            state = self.shared.finished.wait(state).expect("pool state");
        }
        state.work = None;
        if let Some(payload) = state.panic.take() {
            drop(state);
            resume_unwind(payload);
        }
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        self.shared.state.lock().expect("pool state").quit = true;
        self.shared.wake.notify_all();
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

/// The next index of the current work, if any is left.
fn take(state: &mut State) -> Option<usize> {
    let work = state.work?;
    (state.next < work.count).then(|| {
        state.next += 1;
        state.next - 1
    })
}

/// Counts a returned call; keeps the first panic.
fn finish(state: &mut State, called: std::thread::Result<()>, shared: &Shared) {
    if let Err(payload) = called {
        state.panic.get_or_insert(payload);
    }
    state.done += 1;
    if state.work.is_some_and(|w| state.done == w.count) {
        shared.finished.notify_all();
    }
}

fn helper(shared: &Shared) {
    let mut state = shared.state.lock().expect("pool state");
    loop {
        if state.quit {
            return;
        }
        let Some(i) = take(&mut state) else {
            state = shared.wake.wait(state).expect("pool state");
            continue;
        };
        let f = state.work.expect("work for an index").f;
        drop(state);
        // SAFETY: see the top of the file; this call is counted below.
        let called = catch_unwind(AssertUnwindSafe(|| unsafe { (*f)(i) }));
        state = shared.state.lock().expect("pool state");
        finish(&mut state, called, shared);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn every_index_runs_once_and_borrows_end_after() {
        let pool = Pool::new(3);
        for count in [0, 1, 5, 64] {
            let seen: Vec<AtomicUsize> = (0..count).map(|_| AtomicUsize::new(0)).collect();
            pool.run(count, &|i| {
                seen[i].fetch_add(1, Ordering::SeqCst);
            });
            assert!(seen.iter().all(|s| s.load(Ordering::SeqCst) == 1));
        }
    }

    /// Threads of this process with `name`'s prefix, alive now.
    fn threads(name: &str) -> usize {
        std::fs::read_dir("/proc/self/task")
            .map(|d| {
                d.flatten()
                    .filter_map(|t| std::fs::read_to_string(t.path().join("comm")).ok())
                    .filter(|c| c.trim().starts_with(name))
                    .count()
            })
            .unwrap_or(0)
    }

    #[test]
    fn after_a_panic_no_call_is_still_running() {
        let pool = Pool::named(4, "pool-panic");
        let running = AtomicUsize::new(0);
        let returned = AtomicUsize::new(0);
        let caught = catch_unwind(AssertUnwindSafe(|| {
            pool.run(32, &|i| {
                running.fetch_add(1, Ordering::SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(2));
                running.fetch_sub(1, Ordering::SeqCst);
                assert!(i != 3, "index three");
                returned.fetch_add(1, Ordering::SeqCst);
            });
        }));
        assert!(caught.is_err(), "the panic reaches the caller");
        // Every other call ran to its end before run returned.
        assert_eq!(running.load(Ordering::SeqCst), 0);
        assert_eq!(returned.load(Ordering::SeqCst), 31);
    }

    /// Waits up to a second for `name`'s thread count to be `want`.
    fn threads_become(name: &str, want: usize) -> usize {
        let start = std::time::Instant::now();
        loop {
            let n = threads(name);
            if n == want || start.elapsed() > std::time::Duration::from_secs(1) {
                return n;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn dropping_the_pool_ends_its_threads() {
        let pool = Pool::named(3, "pool-drop");
        pool.run(6, &|_| {});
        // A thread takes its name as it starts.
        assert_eq!(threads_become("pool-drop", 3), 3);
        drop(pool);
        // Joined threads leave the task list as the kernel reaps them.
        assert_eq!(threads_become("pool-drop", 0), 0, "drop joins every helper");
    }

    #[test]
    fn a_panic_in_a_call_reaches_the_caller_and_the_pool_goes_on() {
        let pool = Pool::new(2);
        let caught = catch_unwind(AssertUnwindSafe(|| {
            pool.run(8, &|i| assert!(i != 5, "index five"));
        }));
        assert!(caught.is_err());
        let total = AtomicUsize::new(0);
        pool.run(8, &|i| {
            total.fetch_add(i, Ordering::SeqCst);
        });
        assert_eq!(total.load(Ordering::SeqCst), 28);
    }
}
