//! Process and Thread Management
//!
//! This module provides process/thread management for the HNX microkernel:
//! - **Process Control Block (PCB)**: Process metadata and state management
//! - **Task Management**: Thread creation, scheduling, and context switching
//! - **System Calls**: Process-related system call handlers
//! - **Process Lifecycle**: Creation, execution, blocking, zombies, reaping
//! - **Process Groups**: PGID and session management
//! - **Ready Queue**: Round-robin scheduling queue

pub mod syscall;
pub mod task;
pub mod spawn;

use core::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
use shared::sync::mutex::Mutex;

pub use spawn::{sys_process_create_empty, sys_mmap_process, sys_process_start};

/// Process state enumeration
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProcState {
    Created,
    Ready,
    Running,
    Blocked,
    Waiting,   // Blocked on wait() syscall
    Zombie,    // Exited but not reaped
    Finished,
}

/// Process Control Block (PCB)
///
/// Contains all metadata needed to manage a process including:
/// - Process ID and parent relationship
/// - Execution state and priority
/// - Memory management (page table base, ASID)
/// - Process group and session membership
#[derive(Clone, Copy)]
pub struct ProcessControlBlock {
    pub pid: u32,
    pub state: ProcState,
    pub priority: u8,         // 0-255, higher value means higher priority
    pub ticks: u64,           // Number of times this process has been scheduled
    pub parent_pid: u32,      // Parent process ID (0 if orphaned/init)
    pub exit_status: i32,     // Exit status (valid when state is Zombie)
    pub pgid: u32,            // Process Group ID
    pub sid: u32,             // Session ID (for later use)
    pub ttbr0_base: usize,    // User page table base for fork
    pub asid: u16,            // Address Space ID
    pub entry_point: usize,   // Process entry point (PC)
    pub stack_pointer: usize, // Process stack pointer (SP)
    pub wakeup_time: u64,     // Timestamp when process should be woken up (0 = no timeout)
}

/// 进程管理器
///
/// 管理所有进程状态，包括进程控制块、就绪队列等。
pub struct ProcessManager {
    /// 系统计时器滴答数
    system_ticks: AtomicU64,
    /// 下一个可用的进程ID
    next_pid: AtomicUsize,
    /// 进程控制块表（固定大小，32个槽位）
    pcb_table: Mutex<[Option<ProcessControlBlock>; 32]>,
    /// 就绪队列（固定大小，32个槽位）
    ready_queue: Mutex<[u32; 32]>,
    /// 就绪队列头部索引
    ready_head: AtomicUsize,
    /// 就绪队列尾部索引
    ready_tail: AtomicUsize,
}

// Global process management state (legacy - will be removed after migration)
static SYSTEM_TICKS: AtomicU64 = AtomicU64::new(0);
static NEXT_PID: AtomicUsize = AtomicUsize::new(1);
static PCB_TABLE: Mutex<[Option<ProcessControlBlock>; 32]> = Mutex::new([const { None }; 32]);
static READY_QUEUE: Mutex<[u32; 32]> = Mutex::new([0; 32]);
static READY_HEAD: AtomicUsize = AtomicUsize::new(0);
static READY_TAIL: AtomicUsize = AtomicUsize::new(0);

/// 全局进程管理器实例（临时，迁移期间使用）
static PROCESS_MANAGER: ProcessManager = ProcessManager::new();

impl ProcessManager {
    /// 创建新的进程管理器
    pub const fn new() -> Self {
        Self {
            system_ticks: AtomicU64::new(0),
            next_pid: AtomicUsize::new(1),
            pcb_table: Mutex::new([const { None }; 32]),
            ready_queue: Mutex::new([0; 32]),
            ready_head: AtomicUsize::new(0),
            ready_tail: AtomicUsize::new(0),
        }
    }

    /// 将进程ID推入就绪队列
    pub fn ready_queue_push(&self, pid: u32) {
        let t = self.ready_tail.load(Ordering::Relaxed);
        self.ready_queue.lock()[t % 32] = pid;
        self.ready_tail.store((t + 1) % (1 << 16), Ordering::Relaxed);
        crate::info!("ready_queue: pushed PID {}, tail={}", pid, (t + 1) % (1 << 16));
    }

    /// 从就绪队列弹出进程ID
    pub fn ready_queue_pop(&self) -> Option<u32> {
        let h = self.ready_head.load(Ordering::Relaxed);
        let t = self.ready_tail.load(Ordering::Relaxed);
        if h == t {
            crate::info!("ready_queue: empty, head={}, tail={}", h, t);
            return None;
        }
        let pid = self.ready_queue.lock()[h % 32];
        self.ready_head.store((h + 1) % (1 << 16), Ordering::Relaxed);
        crate::info!("ready_queue: popped PID {}, head={}", pid, (h + 1) % (1 << 16));
        Some(pid)
    }

    /// 创建新进程
    pub fn create_process(&self, priority: u8) -> Option<u32> {
        let pid = self.next_pid.fetch_add(1, Ordering::Relaxed) as u32;

        let pcb = ProcessControlBlock {
            pid,
            state: ProcState::Created,
            priority,
            ticks: 0,
            parent_pid: 0,     // 0 means no parent (init process)
            exit_status: 0,
            pgid: pid,         // By default, process is its own group leader
            sid: pid,          // By default, process is its own session leader
            ttbr0_base: 0,
            asid: 0,
            entry_point: 0,    // Will be set when process is started
            stack_pointer: 0,  // Will be set when process is started
            wakeup_time: 0,
        };

        let mut table = self.pcb_table.lock();
        let idx = (pid as usize) % table.len();
        table[idx] = Some(pcb);

        Some(pid)
    }
}

// Re-export commonly used types
pub use task::{Task, TaskState, TaskContext, TaskId, Asid, allocate_asid};

/// Initialize process management subsystem
pub fn init() {
    crate::info!("process: initializing subsystem");
    
    // Initialize IPC system (used for process communication)
    crate::core::ipc::init();
    crate::info!("process: IPC system initialized");
}

/// IPC message handler (backward compatibility)
pub fn ipc_handler(msg: &crate::core::ipc::IpcMessage) -> crate::core::ipc::IpcResponse {
    // Placeholder for routing IPC messages to process handlers
    crate::core::ipc::IpcResponse {
        msg_id: 0,
        code: 0,
        data_len: 0,
        data: [0; 256],
    }
}

// ===== Ready Queue Management =====

pub fn ready_queue_push(pid: u32) {
    PROCESS_MANAGER.ready_queue_push(pid);
}

pub fn ready_queue_pop() -> Option<u32> {
    PROCESS_MANAGER.ready_queue_pop()
}

// ===== Process Creation =====

/// Create a new process with specified priority
pub fn create_process(priority: u8) -> Option<u32> {
    PROCESS_MANAGER.create_process(priority)
}

/// Create a child process (for fork)
pub fn create_child_process(parent_pid: u32, priority: u8, ttbr0_base: usize, asid: u16) -> Option<u32> {
    let pid = NEXT_PID.fetch_add(1, Ordering::Relaxed) as u32;
    
    // Get parent's PGID and SID
    let table = PCB_TABLE.lock();
    let parent_idx = (parent_pid as usize) % table.len();
    let (pgid, sid) = if let Some(parent_pcb) = table[parent_idx] {
        (parent_pcb.pgid, parent_pcb.sid)
    } else {
        (pid, pid) // Orphaned process becomes its own group/session leader
    };
    drop(table);
    
    let pcb = ProcessControlBlock {
        pid,
        state: ProcState::Created,
        priority,
        ticks: 0,
        parent_pid,
        exit_status: 0,
        pgid,              // Inherit parent's process group
        sid,               // Inherit parent's session
        ttbr0_base,
        asid,
        entry_point: 0,    // Will be set when process is started
        stack_pointer: 0,  // Will be set when process is started
        wakeup_time: 0,
    };
    
    let mut table = PCB_TABLE.lock();
    let idx = (pid as usize) % table.len();
    table[idx] = Some(pcb);
    
    Some(pid)
}

/// Spawn a kernel task (no user space)
pub fn spawn_kernel_task(_entry: fn() -> !) -> u32 {
    let pid = NEXT_PID.fetch_add(1, Ordering::Relaxed) as u32;
    {
        let pcb = ProcessControlBlock {
            pid,
            state: ProcState::Created,
            priority: 128, // Medium priority
            ticks: 0,
            parent_pid: 0,
            exit_status: 0,
            pgid: pid,
            sid: pid,
            ttbr0_base: 0,
            asid: 0,
            entry_point: 0,    // Will be set when process is started
            stack_pointer: 0,  // Will be set when process is started
            wakeup_time: 0,
        };
        
        let mut table = PCB_TABLE.lock();
        let idx = (pid as usize) % table.len();
        table[idx] = Some(pcb);
    }
    ready_queue_push(pid);
    pid
}

// ===== Process State Management =====

/// Wake a blocked process
pub fn wake_process(pid: usize) -> bool {
    let mut table = PCB_TABLE.lock();
    let i = pid % table.len();
    if let Some(ref mut pcb) = table[i] {
        if pcb.state == ProcState::Blocked {
            pcb.state = ProcState::Ready;
            drop(table);
            ready_queue_push(pid as u32);
            true
        } else {
            false
        }
    } else {
        false
    }
}

/// Block a running process with optional timeout (0 = no timeout)
pub fn block_process_timeout(pid: usize, timeout_ticks: u64) -> bool {
    let mut table = PCB_TABLE.lock();
    let i = pid % table.len();
    if let Some(ref mut pcb) = table[i] {
        if pcb.state == ProcState::Running {
            pcb.state = ProcState::Blocked;
            if timeout_ticks > 0 {
                pcb.wakeup_time = SYSTEM_TICKS.load(Ordering::Relaxed) + timeout_ticks;
            } else {
                pcb.wakeup_time = 0;
            }
            true
        } else {
            false
        }
    } else {
        false
    }
}

/// Block a running process indefinitely
pub fn block_process(pid: usize) -> bool {
    block_process_timeout(pid, 0)
}

/// Get process state
pub fn get_process_state(pid: usize) -> Option<ProcState> {
    let table = PCB_TABLE.lock();
    let i = pid % table.len();
    table[i].map(|pcb| pcb.state)
}

/// Set process state
pub fn set_process_state(pid: usize, state: ProcState) -> bool {
    let mut table = PCB_TABLE.lock();
    let i = pid % table.len();
    if let Some(ref mut pcb) = table[i] {
        let old_state = pcb.state;
        pcb.state = state;

        // If transitioning to Ready state, add to ready queue
        if old_state != ProcState::Ready && state == ProcState::Ready {
            drop(table); // Release lock before calling ready_queue_push
            crate::info!("process: PID {} added to ready queue", pid);
            ready_queue_push(pid as u32);
        } else {
            crate::debug!("process: PID {} state changed from {:?} to {:?}", pid, old_state, state);
        }
        true
    } else {
        false
    }
}

// ===== Process Lifecycle Management =====

/// Get parent PID
pub fn get_parent_pid(pid: usize) -> Option<u32> {
    let table = PCB_TABLE.lock();
    let idx = pid % table.len();
    table[idx].map(|pcb| pcb.parent_pid)
}

/// Set exit status and transition to zombie state
pub fn set_exit_status(pid: usize, status: i32) -> bool {
    let mut table = PCB_TABLE.lock();
    let idx = pid % table.len();
    if let Some(ref mut pcb) = table[idx] {
        pcb.exit_status = status;
        pcb.state = ProcState::Zombie;
        true
    } else {
        false
    }
}

/// Get exit status (for wait)
pub fn get_exit_status(pid: usize) -> Option<i32> {
    let table = PCB_TABLE.lock();
    let idx = pid % table.len();
    table[idx].map(|pcb| pcb.exit_status)
}

/// Reap zombie process - fully free PCB
pub fn reap_zombie(pid: usize) -> bool {
    let mut table = PCB_TABLE.lock();
    let idx = pid % table.len();
    if let Some(pcb) = table[idx] {
        if pcb.state == ProcState::Zombie {
            table[idx] = None;
            return true;
        }
    }
    false
}

/// Find any zombie child of a parent
pub fn find_zombie_child(parent_pid: u32) -> Option<(u32, i32)> {
    let table = PCB_TABLE.lock();
    for entry in table.iter() {
        if let Some(pcb) = entry {
            if pcb.parent_pid == parent_pid && pcb.state == ProcState::Zombie {
                return Some((pcb.pid, pcb.exit_status));
            }
        }
    }
    None
}

/// Check if process has any children
pub fn has_children(parent_pid: u32) -> bool {
    let table = PCB_TABLE.lock();
    for entry in table.iter() {
        if let Some(pcb) = entry {
            if pcb.parent_pid == parent_pid {
                return true;
            }
        }
    }
    false
}

// ===== Process Group Management =====

/// Get process group ID
pub fn get_pgid(pid: usize) -> Option<u32> {
    let table = PCB_TABLE.lock();
    let idx = pid % table.len();
    table[idx].map(|pcb| pcb.pgid)
}

/// Set process group ID
pub fn set_pgid(pid: usize, pgid: u32) -> bool {
    let mut table = PCB_TABLE.lock();
    let idx = pid % table.len();
    if let Some(ref mut pcb) = table[idx] {
        pcb.pgid = pgid;
        true
    } else {
        false
    }
}

// ===== Process Memory Management =====

/// Update PCB with TTBR0 and ASID (for fork)
pub fn update_process_memory(pid: usize, ttbr0_base: usize, asid: u16) -> bool {
    let mut table = PCB_TABLE.lock();
    let idx = pid % table.len();
    if let Some(ref mut pcb) = table[idx] {
        pcb.ttbr0_base = ttbr0_base;
        pcb.asid = asid;
        true
    } else {
        false
    }
}

// ===== Timeout Management =====

/// Get current system ticks
pub fn get_current_ticks() -> u64 {
    SYSTEM_TICKS.load(Ordering::Relaxed)
}

/// Increment system ticks (called from timer interrupt)
pub fn increment_ticks() {
    SYSTEM_TICKS.fetch_add(1, Ordering::Relaxed);
}

/// Check for timed-out processes and wake them
pub fn check_timeouts() {
    let current_ticks = get_current_ticks();
    let mut table = PCB_TABLE.lock();
    for slot in table.iter_mut() {
        if let Some(ref mut pcb) = slot {
            if pcb.state == ProcState::Blocked && pcb.wakeup_time > 0 && pcb.wakeup_time <= current_ticks {
                pcb.state = ProcState::Ready;
                pcb.wakeup_time = 0;
                ready_queue_push(pcb.pid);
            }
        }
    }
}

/// Timer tick handler (call from timer interrupt)
pub fn timer_tick() {
    increment_ticks();
    check_timeouts();
}

// ===== Scheduling Support =====

/// Update process priority
pub fn set_process_priority(pid: usize, priority: u8) -> bool {
    let mut table = PCB_TABLE.lock();
    let i = pid % table.len();
    if let Some(ref mut pcb) = table[i] {
        pcb.priority = priority;
        true
    } else {
        false
    }
}

/// Increment process tick count
pub fn increment_process_ticks(pid: usize) -> bool {
    let mut table = PCB_TABLE.lock();
    let i = pid % table.len();
    if let Some(ref mut pcb) = table[i] {
        pcb.ticks += 1;
        true
    } else {
        false
    }
}

/// Round-robin tick handler
pub fn on_rr_tick() {
    if let Some(pid) = ready_queue_pop() {
        // Move process to the end of the queue for round-robin scheduling
        ready_queue_push(pid);
    }
}

/// Get process information for scheduling
pub fn get_process_for_scheduling(pid: u32) -> Option<(usize, usize, usize, u16)> {
    let table = PCB_TABLE.lock();
    let idx = (pid as usize) % table.len();
    table[idx].map(|pcb| {
        let mut asid = pcb.asid as u16;
        if asid == 0 {
            asid = pid as u16;
        }
        (
            pcb.entry_point,
            pcb.stack_pointer,
            pcb.ttbr0_base,
            asid
        )
    })
}

/// Update process context (PC and SP)
///
/// This should be called during context switching to save the current
/// execution point before switching away.
pub fn update_process_context(pid: u32, pc: usize, sp: usize) -> bool {
    let mut table = PCB_TABLE.lock();
    let idx = (pid as usize) % table.len();
    if let Some(ref mut pcb) = table[idx] {
        pcb.entry_point = pc;
        pcb.stack_pointer = sp;
        true
    } else {
        false
    }
}

/// Debug function to print ready queue state
pub fn debug_print_ready_queue() {
    let h = READY_HEAD.load(Ordering::Relaxed);
    let t = READY_TAIL.load(Ordering::Relaxed);
    let queue = READY_QUEUE.lock();

    crate::info!("[DEBUG] Ready queue state: head={}, tail={}, size={}", h, t, if h <= t { t - h } else { 32 - (h - t) });
    crate::info!("[DEBUG] Queue contents (indices {} to {}):", h % 32, t % 32);

    let mut current = h;
    while current != t {
        let idx = current % 32;
        let pid = queue[idx];
        if pid != 0 {
            crate::info!("[DEBUG]   index {}: PID {}", idx, pid);
        }
        current = (current + 1) % (1 << 16);
    }

    if h == t {
        crate::info!("[DEBUG]   queue is empty");
    }
}