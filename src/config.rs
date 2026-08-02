//! Global configuration and state management for fault injection.

use crate::types::Operation;
use alloc::vec::Vec;

/// A lightweight spinlock for `no_std` environments.
pub(crate) struct SpinLock<T> {
    locked: core::sync::atomic::AtomicBool,
    data: core::cell::UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for SpinLock<T> {}
unsafe impl<T: Send> Send for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub(crate) const fn new(data: T) -> Self {
        Self {
            locked: core::sync::atomic::AtomicBool::new(false),
            data: core::cell::UnsafeCell::new(data),
        }
    }

    pub(crate) fn lock(&self) -> SpinLockGuard<'_, T> {
        while self
            .locked
            .compare_exchange_weak(
                false,
                true,
                core::sync::atomic::Ordering::Acquire,
                core::sync::atomic::Ordering::Relaxed,
            )
            .is_err()
        {
            core::hint::spin_loop();
        }
        SpinLockGuard { lock: self }
    }

    /// Attempt to acquire the lock without spinning. Returns `None` if it is
    /// already held (by any thread, including the current one).
    ///
    /// This is the reentrancy-safe entry used by [`should_fail_alloc`]: because
    /// the lock is NOT reentrant, a fault-injecting allocation performed while
    /// this thread already holds it (e.g. `Vec::push` reallocating inside
    /// `try_inject_global`) would deadlock if it spun. `try_lock` lets that
    /// nested/contended allocation observe "held" and skip fault evaluation
    /// instead of hanging.
    pub(crate) fn try_lock(&self) -> Option<SpinLockGuard<'_, T>> {
        if self
            .locked
            .compare_exchange(
                false,
                true,
                core::sync::atomic::Ordering::Acquire,
                core::sync::atomic::Ordering::Relaxed,
            )
            .is_ok()
        {
            Some(SpinLockGuard { lock: self })
        } else {
            None
        }
    }
}

pub(crate) struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<T> core::ops::Deref for SpinLockGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> core::ops::DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock
            .locked
            .store(false, core::sync::atomic::Ordering::Release);
    }
}

/// Generates a simple pseudorandom float in `[0.0, 1.0)`.
fn random_f64() -> f64 {
    static SEED: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(1);
    let mut current = SEED.load(core::sync::atomic::Ordering::Relaxed);
    let mut next;
    loop {
        next = current;
        if next == 0 {
            next = 1;
        }
        next ^= next << 13;
        next ^= next >> 7;
        next ^= next << 17;
        match SEED.compare_exchange_weak(
            current,
            next,
            core::sync::atomic::Ordering::Relaxed,
            core::sync::atomic::Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(c) => {
                current = c;
                core::hint::spin_loop();
            }
        }
    }
    let val = next >> 11;
    #[allow(clippy::cast_precision_loss)]
    {
        val as f64 / (1u64 << 53) as f64
    }
}

#[derive(Clone, Debug)]
pub(crate) struct OpState {
    pub(crate) calls: u64,
    pub(crate) fail_points: Vec<u64>,
    pub(crate) probability: f64,
    pub(crate) persist_after: Option<u64>,
}

impl OpState {
    pub(crate) const fn new() -> Self {
        Self {
            calls: 0,
            fail_points: Vec::new(),
            probability: 0.0,
            persist_after: None,
        }
    }

    /// True when this op has any fault configured. `probability <= 0.0` covers
    /// both an unset (0.0) and a negative probability; a NaN probability is NOT
    /// `<= 0.0`, so it counts as active but never actually fires in `check`.
    pub(crate) fn is_active(&self) -> bool {
        !self.fail_points.is_empty() || self.probability > 0.0 || self.persist_after.is_some()
    }

    pub(crate) fn check(&mut self) -> bool {
        // Do NOT consume a call count for an op that has no fault configured:
        // the `calls` index only means something relative to injected faults, so
        // incrementing it while idle (because ENABLED is globally true for some
        // OTHER op) would desync every later `fail_after` offset.
        if !self.is_active() {
            return false;
        }

        let current = self.calls;
        self.calls = self.calls.wrapping_add(1);

        // Always resolve (and consume) the discrete fail point for this call
        // index, even if a probabilistic/persistent trigger also fires. The old
        // code returned early on a prob/persist hit and left the matching
        // discrete point in the vec forever (it could never fire again since
        // `calls` is monotonic) - an unbounded leak of stale fail points.
        let discrete_hit = if let Some(idx) = self.fail_points.iter().position(|&p| p == current) {
            self.fail_points.remove(idx);
            true
        } else {
            false
        };

        let probabilistic_hit = self.probability > 0.0 && random_f64() < self.probability;

        let persistent_hit = self.persist_after.is_some_and(|p| current >= p);

        discrete_hit || probabilistic_hit || persistent_hit
    }
}

pub(crate) struct GlobalState {
    pub(crate) mmap: OpState,
    pub(crate) read: OpState,
    pub(crate) write: OpState,
    pub(crate) alloc: OpState,
    pub(crate) send: OpState,
}

impl GlobalState {
    pub(crate) const fn new() -> Self {
        Self {
            mmap: OpState::new(),
            read: OpState::new(),
            write: OpState::new(),
            alloc: OpState::new(),
            send: OpState::new(),
        }
    }

    pub(crate) fn get_mut(&mut self, op: Operation) -> &mut OpState {
        match op {
            Operation::Mmap => &mut self.mmap,
            Operation::Read => &mut self.read,
            Operation::Write => &mut self.write,
            Operation::Alloc => &mut self.alloc,
            Operation::Send => &mut self.send,
        }
    }

    /// True when ANY operation has a fault configured. Used to recompute the
    /// global `ENABLED` flag after a selective (single-op) state change, e.g. a
    /// scoped guard restoring only its own operation on drop.
    pub(crate) fn any_active(&self) -> bool {
        self.mmap.is_active()
            || self.read.is_active()
            || self.write.is_active()
            || self.alloc.is_active()
            || self.send.is_active()
    }
}

/// Process-global fault state, used by [`inject_global`] for cross-thread
/// injection scenarios.
pub(crate) static ENABLED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);
pub(crate) static STATE: SpinLock<GlobalState> = SpinLock::new(GlobalState::new());

// Thread-local fault state. This is the default target for [`inject`] and
// [`inject_scoped`]: a fault injected on one thread is invisible to other
// threads, so parallel tests do not poison each other.
thread_local! {
    pub(crate) static THREAD_STATE: core::cell::RefCell<GlobalState> =
        const { core::cell::RefCell::new(GlobalState::new()) };
}

/// True if the current thread has any active thread-local fault.
pub(crate) fn thread_local_active() -> bool {
    THREAD_STATE.with(|s| s.borrow().any_active())
}

/// Run `f` with a mutable reference to the current thread's fault state.
pub(crate) fn with_thread_local_state_mut<R>(f: impl FnOnce(&mut GlobalState) -> R) -> R {
    THREAD_STATE.with(|s| f(&mut *s.borrow_mut()))
}

