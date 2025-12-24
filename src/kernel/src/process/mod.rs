pub mod syscall;
pub mod task;

use crate::console;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

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

// Process Control Block structure
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
}

static NEXT_PID: AtomicUsize = AtomicUsize::new(1);
static PCB_TABLE: Mutex<[Option<ProcessControlBlock>; 32]> = Mutex::new([const { None }; 32]);
static READY_QUEUE: Mutex<[u32; 32]> = Mutex::new([0; 32]);
static READY_HEAD: AtomicUsize = AtomicUsize::new(0);
static READY_TAIL: AtomicUsize = AtomicUsize::new(0);

pub fn init() {
    crate::info!("process init");
    // Initialize the enhanced IPC system
    crate::core::ipc::init();
    crate::info!("IPC system initialized");
    
    // For compatibility, we could register a handler for the old IPC system
    // but we'll focus on the new enhanced system
}

pub fn ipc_handler(msg: &crate::core::ipc::IpcMessage) -> crate::core::ipc::IpcResponse {
    // This is a placeholder for backward compatibility
    // In a real implementation, we would route to appropriate handlers
    crate::core::ipc::IpcResponse {
        msg_id: 0,
        code: 0,
        data_len: 0,
        data: [0; 256],
    }
}

fn ready_queue_push(pid: u32) {
    let t = READY_TAIL.load(Ordering::Relaxed);
    READY_QUEUE.lock()[t % 32] = pid;
    READY_TAIL.store((t + 1) % (1 << 16), Ordering::Relaxed);
}

fn ready_queue_pop() -> Option<u32> {
    let h = READY_HEAD.load(Ordering::Relaxed);
    let t = READY_TAIL.load(Ordering::Relaxed);
    if h == t {
        return None;
    }
    let pid = READY_QUEUE.lock()[h % 32];
    READY_HEAD.store((h + 1) % (1 << 16), Ordering::Relaxed);
    Some(pid)
}

// Enhanced process creation with proper PCB initialization
pub fn create_process(priority: u8) -> Option<u32> {
    let pid = NEXT_PID.fetch_add(1, Ordering::Relaxed) as u32;
    
    // Initialize PCB
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
    };
    
    // Store PCB in table
    let mut table = PCB_TABLE.lock();
    let idx = (pid as usize) % table.len();
    table[idx] = Some(pcb);
    
    Some(pid)
}

// Create process with parent (for fork)
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
    
    // Initialize PCB
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
    };
    
    // Store PCB in table
    let mut table = PCB_TABLE.lock();
    let idx = (pid as usize) % table.len();
    table[idx] = Some(pcb);
    
    Some(pid)
}

// Get parent PID
pub fn get_parent_pid(pid: usize) -> Option<u32> {
    let table = PCB_TABLE.lock();
    let idx = pid % table.len();
    table[idx].map(|pcb| pcb.parent_pid)
}

// Set exit status and transition to zombie state
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

// Get exit status (for wait)
pub fn get_exit_status(pid: usize) -> Option<i32> {
    let table = PCB_TABLE.lock();
    let idx = pid % table.len();
    table[idx].map(|pcb| pcb.exit_status)
}

// Reap zombie process - fully free PCB
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

// Find any zombie child of a parent
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

// Check if process has any children
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

// Get/Set process group ID
pub fn get_pgid(pid: usize) -> Option<u32> {
    let table = PCB_TABLE.lock();
    let idx = pid % table.len();
    table[idx].map(|pcb| pcb.pgid)
}

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

// Update PCB with TTBR0 and ASID (for fork)
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

pub fn block_process(pid: usize) -> bool {
    let mut table = PCB_TABLE.lock();
    let i = pid % table.len();
    if let Some(ref mut pcb) = table[i] {
        if pcb.state == ProcState::Running {
            pcb.state = ProcState::Blocked;
            true
        } else {
            false
        }
    } else {
        false
    }
}

// Get process state
pub fn get_process_state(pid: usize) -> Option<ProcState> {
    let table = PCB_TABLE.lock();
    let i = pid % table.len();
    table[i].map(|pcb| pcb.state)
}

// Set process state
pub fn set_process_state(pid: usize, state: ProcState) -> bool {
    let mut table = PCB_TABLE.lock();
    let i = pid % table.len();
    if let Some(ref mut pcb) = table[i] {
        pcb.state = state;
        true
    } else {
        false
    }
}

// Update process priority
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

// Increment process tick count
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

pub fn on_rr_tick() {
    if let Some(pid) = ready_queue_pop() {
        // Move process to the end of the queue for round-robin scheduling
        ready_queue_push(pid);
    }
}

pub fn spawn_kernel_task(_entry: fn() -> !) -> u32 {
    let pid = NEXT_PID.fetch_add(1, Ordering::Relaxed) as u32;
    {
        // Initialize PCB with default values
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
        };
        
        let mut table = PCB_TABLE.lock();
        let idx = (pid as usize) % table.len();
        table[idx] = Some(pcb);
    }
    ready_queue_push(pid);
    pid
}