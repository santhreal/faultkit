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

    pub(crate) fn check(&mut self) -> bool {
        let current = self.calls;
        self.calls = self.calls.wrapping_add(1);

        if self.probability > 0.0 && random_f64() < self.probability {
            return true;
        }

        if let Some(p) = self.persist_after {
            if current >= p {
                return true;
            }
        }

        if let Some(idx) = self.fail_points.iter().position(|&p| p == current) {
            self.fail_points.remove(idx);
            return true;
        }

        false
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
    pub(crate) fn get_mut(&mut self, op: Operation) -> &mut OpState {
        match op {
            Operation::Mmap => &mut self.mmap,
            Operation::Read => &mut self.read,
            Operation::Write => &mut self.write,
            Operation::Alloc => &mut self.alloc,
            Operation::Send => &mut self.send,
        }
    }
}

pub(crate) static ENABLED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);
pub(crate) static STATE: SpinLock<GlobalState> = SpinLock::new(GlobalState {
    mmap: OpState::new(),
    read: OpState::new(),
    write: OpState::new(),
    alloc: OpState::new(),
    send: OpState::new(),
});
