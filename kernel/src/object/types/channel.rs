//! 通道对象
//!
//! 通道对象用于进程间通信（IPC），支持双向消息传递和句柄传递。
//! 通道总是成对创建，每个端点可以独立地发送和接收消息。

use super::event::Event;
use crate::impl_kernel_object;
use crate::object::traits::*;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use shared::sync::mutex::Mutex;

/// 通道消息
pub struct Message {
    /// 消息数据
    data: Vec<u8>,
    /// 传递的句柄
    handles: Vec<crate::object::Handle>,
}

/// 通道对象
pub struct Channel {
    /// 对象ID
    id: KObjectId,
    /// 最大消息大小
    max_msg_size: usize,
    /// 最大句柄数量
    max_msg_handles: usize,
    /// 对端通道（如果存在）
    peer: Mutex<Option<Arc<Channel>>>,
    /// 待处理消息队列
    messages: Mutex<VecDeque<Message>>,
    /// 等待读取的线程（简化实现）
    waiters: Mutex<Vec<Arc<dyn Dispatcher>>>,
}

impl Channel {
    /// 创建一对相互连接的通道
    pub fn create_pair() -> (Arc<Self>, Arc<Self>) {
        let chan1 = Arc::new(Self {
            id: KObjectId::new(),
            max_msg_size: 65536,
            max_msg_handles: 64,
            peer: Mutex::new(None),
            messages: Mutex::new(VecDeque::new()),
            waiters: Mutex::new(Vec::new()),
        });

        let chan2 = Arc::new(Self {
            id: KObjectId::new(),
            max_msg_size: 65536,
            max_msg_handles: 64,
            peer: Mutex::new(Some(chan1.clone())),
            messages: Mutex::new(VecDeque::new()),
            waiters: Mutex::new(Vec::new()),
        });

        // 设置对端的peer引用
        *chan1.peer.lock() = Some(chan2.clone());

        (chan1, chan2)
    }

    /// 发送消息到对端通道
    pub fn write(
        &self,
        data: &[u8],
        handles: Vec<crate::object::Handle>,
    ) -> Result<(), ObjectError> {
        // 检查消息大小
        if data.len() > self.max_msg_size {
            return Err(ObjectError::InvalidArgs);
        }

        // 检查句柄数量
        if handles.len() > self.max_msg_handles {
            return Err(ObjectError::InvalidArgs);
        }

        // 获取对端通道
        let peer = self
            .peer
            .lock()
            .as_ref()
            .cloned()
            .ok_or(ObjectError::PermissionDenied)?; // 对端已关闭

        // 创建消息
        let message = Message {
            data: data.to_vec(),
            handles,
        };

        // 将消息添加到对端消息队列
        peer.messages.lock().push_back(message);

        // 唤醒等待对端消息的线程
        let mut waiters = peer.waiters.lock();
        for waiter in waiters.drain(..) {
            waiter.on_unblock();
        }

        Ok(())
    }

    /// 从通道读取消息
    pub fn read(
        &self,
        buffer: &mut [u8],
        max_handles: usize,
    ) -> Result<(usize, Vec<crate::object::Handle>), ObjectError> {
        let mut messages = self.messages.lock();

        if let Some(msg) = messages.pop_front() {
            // 复制数据到缓冲区
            let copy_len = msg.data.len().min(buffer.len());
            buffer[..copy_len].copy_from_slice(&msg.data[..copy_len]);

            // 处理句柄（限制数量）
            let handles = if msg.handles.len() > max_handles {
                // 如果缓冲区不足，返回错误
                // 简化处理：只返回能容纳的部分
                msg.handles.into_iter().take(max_handles).collect()
            } else {
                msg.handles
            };

            Ok((copy_len, handles))
        } else {
            Err(ObjectError::WouldBlock)
        }
    }

    /// 获取对端通道（如果存在）
    pub fn peer(&self) -> Option<Arc<Channel>> {
        self.peer.lock().as_ref().cloned()
    }

    /// 关闭通道
    pub fn close(&self) {
        // 清空对端引用
        if let Some(peer) = self.peer.lock().take() {
            // 同时清空对端的peer引用（避免循环引用）
            *peer.peer.lock() = None;

            // 唤醒所有等待的线程（返回错误）
            let mut waiters = peer.waiters.lock();
            for waiter in waiters.drain(..) {
                waiter.on_unblock();
            }
        }

        // 清空本地消息队列
        self.messages.lock().clear();
    }
}

impl_kernel_object!(Channel, "Channel");

impl Dispatcher for Channel {
    fn can_block(&self) -> bool {
        true
    }

    fn on_block(&self) {
        // 当线程开始等待此通道时调用
        // 实际实现需要记录等待的线程
    }

    fn on_unblock(&self) {
        // 当通道有消息到达时调用
        // 实际实现需要唤醒等待的线程
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        // 确保通道被正确关闭
        self.close();
    }
}
