use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64},
    },
};

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use safe_lock::LockImmediate;
use std::sync::atomic::Ordering::Relaxed;

pub struct IdRwLock<'a, T> {
    lock: &'a RwLock<T>,
    queue: Arc<RwLock<VecDeque<Box<dyn FnOnce()>>>>,
}

pub struct IdRwLockWriteGuard<'a, T> {
    guard: Option<RwLockWriteGuard<'a, T>>,
    queue: Arc<RwLock<VecDeque<Box<dyn FnOnce()>>>>,
}

pub struct IdRwLockReadGuard<'a, T> {
    guard: RwLockReadGuard<'a, T>,
    queue: Arc<RwLock<VecDeque<Box<dyn FnOnce()>>>>,
}

impl<'a, T> IdRwLock<'a, T> {
    fn try_write(
        &self,
        fallback: impl FnOnce() + 'static,
    ) -> Result<IdRwLockWriteGuard<'a, T>, ()> {
        if let Some(guard) = self.lock.try_write() {
            Ok(IdRwLockWriteGuard {
                guard: Some(guard),
                queue: self.queue.clone(),
            })
        } else {
            self.queue.write().push_back(Box::new(fallback));
            Err(())
        }
    }
}

impl<'a, T> Drop for IdRwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        let guard = self.guard.take().unwrap();
        drop(guard);
        for t in self.queue.write().drain(..) {
            t()
        }
    }
}

// impl<'a, T>IdRwLock<'a, T> {
//     pub fn new(lock: &'a RwLock<T>, id: u64) -> Self {
//         Self { lock, id }
//     }
// }

// impl<'a, T> LockImmediate for IdRwLock<'a, T> {
//     type Error = u64;
//     type Guard = <&'a RwLock<T> as LockImmediate>::Guard;

//     fn lock_immediate(&self) -> Result<Self::Guard, Self::Error> {
//         self.lock.try_write()
//     }
// }
