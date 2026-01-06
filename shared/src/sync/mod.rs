#!/! # HNX 同步原语
//! # HNX Sync Primitives
//!
//! 这个模块提供了用于内核和用户空间的核心同步原语，
//! This module provides core synchronization primitives for both kernel and user space,
//! 例如 `Mutex`、`Semaphore` 和 `RwLock`。
//! such as `Mutex`, `Semaphore`, and `RwLock`.
//!
//! 这些原语被设计为 `no_std` 环境兼容，并为 HNX 的多进程模型提供基础。
//! These primitives are designed to be `no_std` compatible and provide the foundation for HNX's multi-process model.

pub mod barrier;
pub mod mutex;
