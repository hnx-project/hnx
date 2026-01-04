//! Process synchronization mechanisms
//!
//! This module implements various synchronization primitives that can be used
//! by processes to coordinate access to shared resources.

use crate::sync::{Mutex, Semaphore, Condvar};
use core::ops::{Deref, DerefMut};

/// A barrier for process synchronization
///
/// A barrier enables multiple processes to synchronize at a certain point
/// in their execution.
pub struct Barrier {
    count: usize,
    current: Mutex<usize>,
    semaphore: Semaphore,
    condvar: Condvar,
}

impl Barrier {
    /// Creates a new barrier that can be used by `count` processes
    pub fn new(count: usize) -> Self {
        Self {
            count,
            current: Mutex::new(0),
            semaphore: Semaphore::new(0),
            condvar: Condvar::new(),
        }
    }
    
    /// Waits for all processes to reach this barrier
    ///
    /// When all processes have called `wait`, they will all proceed together.
    pub fn wait(&self) {
        let mut current = self.current.lock();
        *current += 1;
        
        if *current == self.count {
            // Last process reached the barrier, wake up all others
            for _ in 0..(self.count - 1) {
                self.semaphore.release();
            }
            *current = 0;
        } else {
            // Drop the lock before waiting
            drop(current);
            // Wait for the semaphore to be released
            self.semaphore.acquire();
        }
    }
}

/// A reader-writer lock implementation
///
/// This lock allows multiple readers or a single writer to access a resource.
pub struct RwLock<T: ?Sized> {
    // Number of readers currently holding the lock
    readers: Mutex<usize>,
    // Writer waiting semaphore
    writer_sem: Semaphore,
    // Writer active flag
    writer_active: Mutex<bool>,
    // Data protected by the lock
    data: T,
}

unsafe impl<T: ?Sized + Send> Send for RwLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    /// Creates a new reader-writer lock
    pub const fn new(data: T) -> Self {
        Self {
            readers: Mutex::new(0),
            writer_sem: Semaphore::new(1),
            writer_active: Mutex::new(false),
            data,
        }
    }
}

impl<T: ?Sized> RwLock<T> {
    /// Acquires a read lock
    ///
    /// Multiple processes can hold a read lock simultaneously.
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        // Acquire writer semaphore to ensure no writer is waiting
        self.writer_sem.acquire();
        
        // Increment reader count
        let mut readers = self.readers.lock();
        *readers += 1;
        
        // Release writer semaphore so other readers can proceed
        self.writer_sem.release();
        
        RwLockReadGuard { lock: self }
    }
    
    /// Acquires a write lock
    ///
    /// Only one process can hold a write lock, and it excludes all readers.
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        // Acquire writer semaphore to prevent new readers
        self.writer_sem.acquire();
        
        // Set writer active flag
        let mut writer_active = self.writer_active.lock();
        *writer_active = true;
        
        // Wait for all readers to finish
        let readers = self.readers.lock();
        while *readers > 0 {
            // In a more advanced implementation, we would wait for a condition variable
            drop(readers);
            core::hint::spin_loop();
            // Reacquire the lock
            // Note: This is not ideal, but sufficient for a basic implementation
        }
        
        RwLockWriteGuard { lock: self }
    }
}

/// A guard that releases a read lock when dropped
pub struct RwLockReadGuard<'a, T: ?Sized> {
    lock: &'a RwLock<T>,
}

impl<'a, T: ?Sized> Deref for RwLockReadGuard<'a, T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        &self.lock.data
    }
}

impl<'a, T: ?Sized> Drop for RwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        let mut readers = self.lock.readers.lock();
        *readers -= 1;
    }
}

/// A guard that releases a write lock when dropped
pub struct RwLockWriteGuard<'a, T: ?Sized> {
    lock: &'a RwLock<T>,
}

impl<'a, T: ?Sized> Deref for RwLockWriteGuard<'a, T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        &self.lock.data
    }
}

impl<'a, T: ?Sized> DerefMut for RwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.lock.data
    }
}

impl<'a, T: ?Sized> Drop for RwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        // Set writer active flag to false
        let mut writer_active = self.lock.writer_active.lock();
        *writer_active = false;
        
        // Release writer semaphore
        self.lock.writer_sem.release();
    }
}