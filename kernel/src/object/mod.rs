//! 内核对象系统
//!
//! 基于能力的对象系统，参考Zircon微内核设计。
//! 核心概念：
//! - 一切皆对象：所有内核资源都表示为对象
//! - 基于句柄：用户态通过句柄引用对象
//! - 基于能力：句柄带有权限，实现最小权限原则
//! - 引用计数：对象生命周期由引用计数管理

pub mod handle;
pub mod table;
pub mod traits;
pub mod types;

pub use handle::Handle;
pub use table::HandleTable;
pub use traits::{
    KernelObject, Dispatcher, DowncastArc, DowncastRef,
    KObjectId, ObjectRights, ObjectSignals, ObjectError,
};

/// 对象系统初始化
pub fn init() {
    // 对象系统初始化逻辑
    // 目前为空，后续可以初始化全局对象表等
}

/// 简化对象实现的宏
#[macro_export]
macro_rules! impl_kernel_object {
    ($type:ty, $name:expr) => {
        impl $crate::object::traits::KernelObject for $type {
            fn type_name(&self) -> &'static str {
                $name
            }

            fn id(&self) -> $crate::object::traits::KObjectId {
                self.id
            }

            fn as_any(&self) -> &dyn core::any::Any {
                self
            }
        }
    };
}