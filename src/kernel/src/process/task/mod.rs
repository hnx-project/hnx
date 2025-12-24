use core::ops::Range;
use core::sync::atomic::{AtomicU16, Ordering};

pub type TaskId = u64;

/// ASID (Address Space ID) type - 16-bit on AArch64
pub type Asid = u16;

/// Global ASID allocator - starts from 1 (0 is reserved for kernel)
static NEXT_ASID: AtomicU16 = AtomicU16::new(1);

/// Maximum ASID value (16-bit, but many implementations support fewer)
const MAX_ASID: u16 = 255; // Conservative limit for compatibility

#[derive(Clone, Copy)]
pub enum TaskState {
    Ready,
    Running,
    Finished,
}

#[derive(Clone, Copy)]
pub struct TaskContext {
    pub sp: usize,
    pub pc: usize,
}

pub struct Task {
    pub id: TaskId,
    pub state: TaskState,
    pub context: TaskContext,
    pub entry_point: usize,
    pub stack: Range<usize>,
    pub ttbr0_base: usize,
    /// Address Space ID for TLB isolation
    pub asid: Asid,
}

impl Task {
    pub fn new_kernel(entry: fn() -> !) -> Self {
        let sp_top = 0x8000_0000usize;
        let sp_bot = sp_top - 0x10000;
        Self {
            id: 1,
            state: TaskState::Ready,
            context: TaskContext {
                sp: sp_top,
                pc: entry as usize,
            },
            entry_point: entry as usize,
            stack: sp_bot..sp_top,
            ttbr0_base: 0,
            asid: allocate_asid(),
        }
    }
}

/// Allocates a unique ASID for a new task/process
/// 
/// # Security Note
/// ASIDs prevent TLB pollution between processes, ensuring that cached
/// translations from one process cannot be used by another, even if
/// virtual addresses overlap.
pub fn allocate_asid() -> Asid {
    let asid = NEXT_ASID.fetch_add(1, Ordering::Relaxed);
    
    // Handle ASID exhaustion with wraparound
    if asid >= MAX_ASID {
        // In a production kernel, this would trigger a TLB flush
        // and reset the allocator. For now, we'll wrap around.
        NEXT_ASID.store(1, Ordering::Relaxed);
        crate::info!("task: ASID wraparound detected, TLB flush required");
        // TODO: Implement global TLB invalidation here
        unsafe {
            core::arch::asm!("tlbi vmalle1; dsb ish; isb");
        }
        return 1;
    }
    
    asid
}