/// 内核对象句柄
/// 
/// 句柄是内核对象的引用，用于在进程之间传递对象引用。
/// 每个句柄都有一个权限位，用于控制对对象的访问。
use super::traits::{KernelObject, ObjectRights, ObjectError};
use alloc::sync::Arc;

/// 内核对象句柄
#[derive(Clone)]
pub struct Handle {
    /// 指向内核对象的引用计数指针
    object: Arc<dyn KernelObject>,
    /// 该句柄拥有的权限
    rights: ObjectRights,
    /// 所属进程ID
    owner_pid: u64,
}

impl Handle {
    pub fn new(object: Arc<dyn KernelObject>, rights: ObjectRights, owner_pid: u64) -> Handle {
        Handle {
            object,
            rights,
            owner_pid,
        }
    }
    
    pub fn object(&self) -> &Arc<dyn KernelObject> {
        &self.object
    }
    
    pub fn rights(&self) -> ObjectRights {
        self.rights
    }
    
    /// 检查是否拥有指定权限
    pub fn check_rights(&self, required: ObjectRights) -> Result<(), ObjectError> {
        if self.rights.contains(required) {
            Ok(())
        } else {
            Err(ObjectError::PermissionDenied)
        }
    }
    
    /// 复制句柄（可选降低权限）
    pub fn duplicate(&self, new_rights: Option<ObjectRights>) -> Handle {
        let rights = new_rights.unwrap_or(self.rights);
        Handle {
            object: self.object.clone(),
            rights,
            owner_pid: self.owner_pid,
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        // 句柄销毁时，对象引用计数减1
        // 当最后一个句柄被销毁，对象也会被销毁
    }
}