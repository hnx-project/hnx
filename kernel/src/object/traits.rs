// 基本内核特征
use core::sync::atomic::{AtomicU32, Ordering};
use alloc::sync::Arc;
use core::any::Any;


/// 内核对象ID类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KObjectId(u64);

impl KObjectId {
    pub fn new() -> KObjectId {
        static NEXT_ID: AtomicU32 = AtomicU32::new(1);
        KObjectId(NEXT_ID.fetch_add(1, Ordering::Relaxed) as u64)
    }
}

bitflags::bitflags! {
    /// 对象权限位
    #[derive(Clone, Copy)]
    pub struct ObjectRights: u32 {
        const DUPLICATE   = 1 << 0;
        const READ        = 1 << 1;
        const WRITE       = 1 << 2;
        const EXECUTE     = 1 << 3;
        const MAP         = 1 << 4;
        const GET_PROPERTY = 1 << 5;
        const SET_PROPERTY = 1 << 6;
        const ENUMERATE   = 1 << 7;
        const DESTROY     = 1 << 8;
        const SAME_RIGHTS = 1 << 31; // 复制时保持相同权限
    }
}


/// 基本信号位
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectSignals(u32);

impl ObjectSignals {
    pub fn empty() -> Self {
        Self(0)
    }

    pub fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    pub fn bits(&self) -> u32 {
        self.0
    }

    pub fn contains(&self, other: ObjectSignals) -> bool {
        (self.0 & other.0) == other.0
    }
}


/// 所有内核对象必须实现的特性
pub trait KernelObject: Send + Sync + Any {
    /// 返回对象类型名
    fn type_name(&self) -> &'static str;

    /// 返回对象ID
    fn id(&self) -> KObjectId;

    /// 对象支持的基本信号
    fn signals(&self) -> ObjectSignals {
        ObjectSignals::empty()
    }

    /// 关闭对象时的清理操作
    fn on_close(&self) {}

    /// 获取对象属性（用于调试等）
    fn get_property(&self, _property: u32, _value: &mut [u8]) -> Result<(), ObjectError> {
        Err(ObjectError::NotSupported)
    }

    /// 将self转换为Any trait对象，以便向下转换
    fn as_any(&self) -> &dyn Any;
}

/// 可调度对象特性
/// 
/// 所有可阻塞等待信号的内核对象都必须实现此特性
pub trait Dispatcher: KernelObject {
    /// 是否可以阻塞等待信号
    fn can_block(&self) -> bool;
    
    /// 当对象阻塞等待信号时调用
    fn on_block(&self);
    
    /// 当对象收到信号时调用
    fn on_unblock(&self);
}

/// 为Arc<dyn KernelObject>提供向下转换方法
pub trait DowncastArc {
    /// 尝试将Arc<dyn KernelObject>向下转换为具体类型
    fn downcast_arc<T: KernelObject + 'static>(self) -> Result<Arc<T>, Self> where Self: Sized;
}

impl DowncastArc for Arc<dyn KernelObject> {
    fn downcast_arc<T: KernelObject + 'static>(self) -> Result<Arc<T>, Self> {
        // 使用Any进行向下转换
        if self.as_any().is::<T>() {
            // 安全：我们已经检查了类型
            unsafe {
                let ptr = Arc::into_raw(self);
                let casted = Arc::from_raw(ptr as *const T);
                Ok(casted)
            }
        } else {
            Err(self)
        }
    }
}

/// 为&dyn KernelObject提供向下转换方法
pub trait DowncastRef {
    /// 尝试将&dyn KernelObject向下转换为具体类型的引用
    fn downcast_ref<T: KernelObject + 'static>(&self) -> Option<&T>;
}

impl DowncastRef for dyn KernelObject {
    fn downcast_ref<T: KernelObject + 'static>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }
}

/// 错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectError {
    NotFound,
    PermissionDenied,
    InvalidArgs,
    BadHandle,
    WrongType,
    NotSupported,
    WouldBlock,
    TimedOut,
    NoMemory,
}