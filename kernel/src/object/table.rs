// src/kernel/object/table.rs

use super::handle::Handle;
use super::traits::{KernelObject, ObjectError, ObjectRights};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
// use spin::Mutex;
use shared::sync::mutex::Mutex;

/// 进程句柄表结构
pub struct HandleTable {
    /// 句柄到对象的映射
    handles: Mutex<Vec<Option<Handle>>>,
    /// 进程ID
    pid: u64,
}
/// 进程句柄表实现
impl HandleTable {
    /// 最大句柄数 （默认1024）
    const MAX_HANDLES: usize = 1024;

    pub fn new(pid: u64) -> Self {
        Self {
            handles: Mutex::new(vec![None; Self::MAX_HANDLES]),
            pid,
        }
    }

    /// 添加句柄，返回句柄值
    pub fn add(
        &self,
        object: Arc<dyn KernelObject>,
        rights: ObjectRights,
    ) -> Result<usize, ObjectError> {
        let mut handles = self.handles.lock();
        // 寻找空闲槽位
        for (idx, slot) in handles.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(Handle::new(object, rights, self.pid));
                return Ok(idx);
            }
        }
        Err(ObjectError::NoMemory)
    }

    /// 获取句柄（不取出）
    pub fn get(&self, handle_value: usize) -> Result<Handle, ObjectError> {
        let handles = self.handles.lock();
        handles
            .get(handle_value)
            .and_then(|h| h.as_ref())
            .cloned()
            .ok_or(ObjectError::BadHandle)
    }

    /// 移除句柄（取出所有权）
    pub fn remove(&self, handle_value: usize) -> Result<Handle, ObjectError> {
        let mut handles = self.handles.lock();
        if let Some(slot) = handles.get_mut(handle_value) {
            slot.take().ok_or(ObjectError::BadHandle)
        } else {
            Err(ObjectError::BadHandle)
        }
    }

    /// 替换句柄（原子操作）
    pub fn replace(&self, handle_value: usize, new_handle: Handle) -> Result<Handle, ObjectError> {
        let mut handles = self.handles.lock();
        if let Some(slot) = handles.get_mut(handle_value) {
            slot.replace(new_handle).ok_or(ObjectError::BadHandle)
        } else {
            Err(ObjectError::BadHandle)
        }
    }
}
