use crate::process::{create_process, set_process_state, ProcState};
use crate::arch::common::mmu::MmuFlags;
use crate::memory::virtual_::{create_user_l1, map_in_pt};
use crate::{info, error};

/// Create an empty user process with a page table
///
/// This is a PURE kernel operation - just create process structure and address space.
/// No code loading, no ELF parsing. That's user space's job.
///
/// Returns (PID, page_table_base) or error
pub fn sys_process_create_empty(priority: u8) -> Result<(u32, usize), ()> {
    let pid = create_process(priority).ok_or_else(|| {
        error!("process: failed to allocate PID");
    })?;
    
    let pt_base = create_user_l1().ok_or_else(|| {
        error!("process: failed to create page table");
    })?;
    
    crate::process::update_process_memory(pid as usize, pt_base, 0);
    
    info!("process: created empty process pid={} pt=0x{:X}", pid, pt_base);
    
    Ok((pid, pt_base))
}

/// Map a physical page into user process address space
///
/// This is called by user space Loader service to populate process memory.
///
/// # Arguments
/// * `pid` - Target process ID
/// * `vaddr` - Virtual address to map to
/// * `paddr` - Physical address to map (0 = allocate new page)
/// * `readable` - Page is readable
/// * `writable` - Page is writable  
/// * `executable` - Page is executable
pub fn sys_mmap_process(
    pid: u32,
    vaddr: usize,
    paddr: usize,
    readable: bool,
    writable: bool,
    executable: bool,
) -> Result<usize, ()> {
    // Get process page table
    // For now, we don't have a way to get pt_base from PCB
    // This needs to be added to process module
    
    let actual_paddr = if paddr == 0 {
        crate::memory::physical::alloc_pages(1).ok_or_else(|| {
            error!("mmap: failed to allocate page");
        })?
    } else {
        paddr
    };
    
    let mut flags = MmuFlags::USER;
    if readable { flags = flags.combine(MmuFlags::READ); }
    if writable { flags = flags.combine(MmuFlags::WRITE); }
    if executable { flags = flags.combine(MmuFlags::EXECUTE); }
    
    // TODO: Get pt_base from process PCB
    // For now this is incomplete
    
    info!("mmap: pid={} vaddr=0x{:X} paddr=0x{:X}", 
          pid, vaddr, actual_paddr);
    
    Ok(actual_paddr)
}

/// Start a user process
///
/// Sets the entry point and stack pointer, then marks process as Ready.
///
/// # Arguments
/// * `pid` - Process ID to start
/// * `entry` - Entry point address (must be mapped)
/// * `sp` - Stack pointer (must be mapped)
pub fn sys_process_start(pid: u32, entry: usize, sp: usize) -> Result<(), ()> {
    // Set entry point and SP in process structure
    let mut table = super::PCB_TABLE.lock();
    let idx = (pid as usize) % table.len();
    if let Some(ref mut pcb) = table[idx] {
        pcb.entry_point = entry;
        pcb.stack_pointer = sp;
    } else {
        return Err(());
    }
    drop(table);

    // Mark process as ready (which will add it to ready queue)
    super::set_process_state(pid as usize, super::ProcState::Ready);

    info!("process: started pid={} entry=0x{:X} sp=0x{:X}", pid, entry, sp);

    Ok(())
}