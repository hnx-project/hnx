//! 具体内核对象类型实现
//!
//! 包含各种内核对象的具体实现：
//! - 事件对象（Event、EventPair）
//! - 通信对象（Channel、Socket）
//! - 内存对象（Vmo）
//! - 进程对象（Process、Job、Thread）
//! - 其他系统对象

pub mod event;
pub mod channel;
pub mod vmo;
pub mod process;
pub mod thread;

// 重新导出常用对象类型
pub use event::{Event, EventPair};
pub use channel::Channel;
pub use vmo::Vmo;
pub use process::Process;
pub use thread::Thread;