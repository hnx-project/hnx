use core::ptr;
use spin::Mutex;
use crate::security::{self, validate_capability, rights};

// 使用 abi 中定义的系统调用号和类型
use hnx_abi::{
    HNX_SYS_WRITE, HNX_SYS_EXIT, HNX_SYS_GETPID, HNX_SYS_OPEN, HNX_SYS_READ, HNX_SYS_CLOSE, HNX_SYS_YIELD,
    HNX_SYS_IPC_WAIT, HNX_SYS_IPC_WAKE, HNX_SYS_EP_CREATE, HNX_SYS_EP_SEND, HNX_SYS_EP_RECV,
    HNX_SYS_DLOPEN, HNX_SYS_DLCLOSE, HNX_SYS_DLSYM, HNX_SYS_CREAT, HNX_SYS_UNLINK, HNX_SYS_MKDIR, HNX_SYS_RMDIR,
    HNX_SYS_MMAP, HNX_SYS_MUNMAP, HNX_SYS_MPROTECT, HNX_SYS_FORK, HNX_SYS_KILL, HNX_SYS_SETPGID, HNX_SYS_GETPGID,
    HNX_SYS_GETPPID, HNX_SYS_WAIT4, HNX_SYS_DRIVER_REGISTER, HNX_SYS_DRIVER_REQUEST_IRQ,
    HNX_SYS_DRIVER_MAP_MMIO, HNX_SYS_DRIVER_DMA_ALLOC, HNX_SYS_SOCKET, HNX_SYS_BIND, HNX_SYS_CONNECT,
    HNX_SYS_LISTEN, HNX_SYS_ACCEPT, HNX_SYS_SEND, HNX_SYS_RECV, SysResult
};

#[derive(Copy, Clone)]
struct CapEntry {
    /// The capability ID that grants access to this endpoint
    cap_id: u32,
    /// The endpoint ID
    ep_id: u32,
    /// Rights granted by this capability
    rights: u8,
}
static CAPS: Mutex<[[Option<CapEntry>; 16]; 32]> = Mutex::new([[None; 16]; 32]);
const RIGHT_SEND: u8 = 0x1;
const RIGHT_RECV: u8 = 0x2;
const RIGHT_MANAGE: u8 = 0x4;

fn caps_alloc_handle(pid: usize, ep_id: u32, rights: u8) -> usize {
    // Allocate a new capability for this endpoint
    let cap_id = if let Some(id) = security::allocate_capability(ep_id, rights as u32) {
        id
    } else {
        return usize::MAX;
    };
    
    let mut tbl = CAPS.lock();
    let idx = pid % 32;
    let row = &mut tbl[idx];
    for i in 0..row.len() {
        if row[i].is_none() {
            row[i] = Some(CapEntry { cap_id, ep_id, rights });
            return i;
        }
    }
    usize::MAX
}

fn caps_lookup(pid: usize, handle: usize) -> Option<CapEntry> {
    let tbl = CAPS.lock();
    let idx = pid % 32;
    let row = &tbl[idx];
    if handle < row.len() {
        row[handle]
    } else {
        None
    }
}

fn copy_from_user(src: usize, len: usize, dst: &mut [u8]) -> usize {
    let n = core::cmp::min(len, dst.len());
    if !user_range_ok(src, n, false) {
        return 0;
    }
    unsafe {
        ptr::copy_nonoverlapping(src as *const u8, dst.as_mut_ptr(), n);
    }
    n
}

fn copy_to_user(dst: usize, src: &[u8]) -> usize {
    if !user_range_ok(dst, src.len(), true) {
        return 0;
    }
    unsafe {
        ptr::copy_nonoverlapping(src.as_ptr(), dst as *mut u8, src.len());
    }
    src.len()
}

fn user_range_ok(addr: usize, len: usize, write: bool) -> bool {
    // CRITICAL SECURITY: Validate user memory accesses
    
    // 1. NULL pointer check
    if addr == 0 {
        crate::debug!("syscall: NULL pointer rejected");
        return false;
    }
    
    // 2. Check for overflow in address + length
    let end = match addr.checked_add(len) {
        Some(e) => e,
        None => {
            crate::debug!("syscall: address overflow rejected");
            return false;
        }
    };
    
    // 3. Ensure address is in user space (< 0xFFFF_8000_0000_0000)
    // Kernel addresses start at KERNEL_BASE (0xFFFF_8000_0000_0000)
    const USER_SPACE_MAX: usize = 0x0000_8000_0000_0000;
    if addr >= USER_SPACE_MAX || end > USER_SPACE_MAX {
        crate::debug!("syscall: kernel address access rejected addr=0x{:X} end=0x{:X}", addr, end);
        return false;
    }
    
    // 4. Get current page table base
    let base = if let Some(b) = crate::kernel_core::scheduler::current_ttbr0_base() {
        b
    } else {
        crate::debug!("syscall: no active page table");
        return false;
    };
    
    // 5. Validate using memory protection module
    if !crate::memory::protection::validate_memory_access(base, addr, len, write) {
        crate::debug!("syscall: memory protection check failed");
        return false;
    }
    
    // 6. Walk page tables to verify mapping and permissions
    let mut a = addr;
    while a < end {
        if !user_page_ok(base, a, write) {
            crate::debug!("syscall: page not accessible addr=0x{:X} write={}", a, write);
            return false;
        }
        a = a.saturating_add(4096 - (a & 0xFFF));
    }
    true
}

fn user_page_ok(pt_base: usize, vaddr: usize, write: bool) -> bool {
    unsafe {
        // SECURITY: Page table walk with comprehensive permission checks
        let l1 = pt_base as *const u64;
        
        // L1 index uses VA[38:30] for 3-level translation (T0SZ=25)
        let l1i = ((vaddr >> 30) & 0x1FF);
        let l1e = core::ptr::read_volatile(l1.add(l1i));
        
        // Check L1 descriptor is valid and is a table descriptor
        if l1e & 0x3 != 3 {
            return false;
        }
        
        let l2_pa = (l1e & !0xFFF) as usize;
        let l2 = l2_pa as *const u64;
        
        // L2 index uses VA[29:21]
        let l2i = ((vaddr >> 21) & 0x1FF);
        let l2e = core::ptr::read_volatile(l2.add(l2i));
        let ty = l2e & 0x3;
        
        if ty == 1 {
            // Block descriptor (2MB block) - check permissions
            return check_page_permissions(l2e, write);
        } else if ty == 3 {
            // Table descriptor - walk to L3
            let l3_pa = (l2e & !0xFFF) as usize;
            let l3 = l3_pa as *const u64;
            let l3i = ((vaddr >> 12) & 0x1FF);
            let l3e = core::ptr::read_volatile(l3.add(l3i));
            
            // Check L3 descriptor is valid
            if l3e & 0x3 != 3 {
                return false;
            }
            
            return check_page_permissions(l3e, write);
        } else {
            false
        }
    }
}

/// Helper function to check page table entry permissions
/// 
/// # Security Checks
/// - Verifies AP bits allow the requested access (read/write)
/// - Ensures UXN bit prevents user execution of kernel pages
/// - Ensures PXN bit prevents kernel execution of user pages
fn check_page_permissions(entry: u64, write: bool) -> bool {
    // Extract AP[7:6] bits
    // AP encoding:
    // 00: EL1 RW, EL0 NA (kernel read-write)
    // 01: EL1 RO, EL0 NA (kernel read-only)
    // 10: EL1 RW, EL0 RW (user read-write)
    // 11: EL1 RO, EL0 RO (user read-only)
    let ap = ((entry >> 6) & 0x3) as u8;
    
    // Check if page is accessible to user (AP[1] == 1)
    if (ap & 0b10) == 0 {
        // Kernel-only page, reject user access
        return false;
    }
    
    // Check write permission
    if write {
        // For write access, AP must be 0b10 (RW for both EL1 and EL0)
        if ap != 0b10 {
            return false;
        }
    }
    
    // Additional security: verify UXN is set (bit 54)
    // This prevents user from executing kernel code
    // For user pages, UXN should be clear only if page is user-executable
    let uxn = (entry & (1u64 << 54)) != 0;
    
    // Additional security: verify PXN is set for user pages (bit 53)
    // This prevents kernel from executing user code
    let pxn = (entry & (1u64 << 53)) != 0;
    
    // For user-accessible pages, PXN should always be set
    if !pxn {
        crate::debug!("syscall: user page without PXN detected - security violation");
        return false;
    }
    
    true
}

pub fn dispatch(
    num: u32,
    x0: usize,
    x1: usize,
    x2: usize,
    _x3: usize,
    _x4: usize,
    _x5: usize,
) -> SysResult {
    match num {
        HNX_SYS_WRITE => {
            crate::debug!("syscall enter write");
            sys_write(x0, x1, x2)
        }
        HNX_SYS_EXIT => {
            crate::debug!("syscall enter exit");
            sys_exit(x0)
        }
        HNX_SYS_GETPID => {
            crate::debug!("syscall enter getpid");
            sys_getpid()
        }
        HNX_SYS_GETPPID => {
            crate::debug!("syscall enter getppid");
            process::sys_getppid()
        }
        HNX_SYS_FORK => {
            crate::debug!("syscall enter fork");
            process::sys_fork()
        }
        HNX_SYS_WAIT4 => {
            crate::debug!("syscall enter wait4");
            process::sys_wait4(x0 as isize, x1, x2, _x3)
        }
        HNX_SYS_KILL => {
            crate::debug!("syscall enter kill");
            signal::sys_kill(x0 as isize, x1 as u32)
        }
        HNX_SYS_SETPGID => {
            crate::debug!("syscall enter setpgid");
            process::sys_setpgid(x0, x1)
        }
        HNX_SYS_GETPGID => {
            crate::debug!("syscall enter getpgid");
            process::sys_getpgid(x0)
        }
        HNX_SYS_OPEN => {
            crate::debug!("syscall enter open");
            sys_open(x0, x1 as u32, x2 as u32)
        }
        HNX_SYS_READ => {
            crate::debug!("syscall enter read");
            sys_read(x0, x1, x2)
        }
        HNX_SYS_CLOSE => {
            crate::debug!("syscall enter close");
            // For now, just call the regular close function
            sys_close(x0)
        }
        HNX_SYS_CREAT => {
            crate::debug!("syscall enter creat");
            sys_creat(x0, x1 as u32)
        }
        HNX_SYS_UNLINK => {
            crate::debug!("syscall enter unlink");
            sys_unlink(x0)
        }
        HNX_SYS_MKDIR => {
            crate::debug!("syscall enter mkdir");
            sys_mkdir(x0, x1 as u32)
        }
        HNX_SYS_RMDIR => {
            crate::debug!("syscall enter rmdir");
            sys_rmdir(x0)
        }
        HNX_SYS_YIELD => {
            crate::kernel_core::scheduler::on_tick();
            0
        }
        HNX_SYS_IPC_WAIT => {
            let _ = crate::process::block_process(
                crate::kernel_core::scheduler::current_pid() as usize
            );
            0
        }
        HNX_SYS_IPC_WAKE => {
            let _ = crate::process::wake_process(x0);
            0
        }
        HNX_SYS_EP_CREATE => {
            crate::debug!("syscall enter ep_create");
            sys_ep_create()
        }
        HNX_SYS_EP_SEND => {
            crate::debug!("syscall enter ep_send");
            sys_ep_send(x0, x1, x2, _x3)
        }
        HNX_SYS_EP_RECV => {
            crate::debug!("syscall enter ep_recv");
            sys_ep_recv(x0, x1, x2)
        }
        HNX_SYS_DRIVER_REGISTER => {
            crate::debug!("syscall enter driver_register");
            driver::sys_driver_register(x0, x1)
        }
        HNX_SYS_DRIVER_REQUEST_IRQ => {
            crate::debug!("syscall enter driver_request_irq");
            driver::sys_driver_request_irq(x0 as u32)
        }
        HNX_SYS_DRIVER_MAP_MMIO => {
            crate::debug!("syscall enter driver_map_mmio");
            driver::sys_driver_map_mmio(x0 as u64, x1)
        }
        HNX_SYS_DRIVER_DMA_ALLOC => {
            crate::debug!("syscall enter driver_dma_alloc");
            driver::sys_driver_dma_alloc(x0, x1)
        }
        HNX_SYS_SOCKET => {
            crate::debug!("syscall enter socket");
            network::sys_socket(x0 as u32, x1 as u32, x2 as u32)
        }
        HNX_SYS_BIND => {
            crate::debug!("syscall enter bind");
            network::sys_bind(x0 as u32, x1, x2)
        }
        HNX_SYS_CONNECT => {
            crate::debug!("syscall enter connect");
            network::sys_connect(x0 as u32, x1, x2)
        }
        HNX_SYS_LISTEN => {
            crate::debug!("syscall enter listen");
            network::sys_listen(x0 as u32, x1 as u32)
        }
        HNX_SYS_ACCEPT => {
            crate::debug!("syscall enter accept");
            network::sys_accept(x0 as u32, x1, x2)
        }
        HNX_SYS_SEND => {
            crate::debug!("syscall enter send");
            network::sys_send(x0 as u32, x1, x2, _x3 as u32)
        }
        HNX_SYS_RECV => {
            crate::debug!("syscall enter recv");
            network::sys_recv(x0 as u32, x1, x2, _x3 as u32)
        }
        HNX_SYS_DLOPEN => {
            crate::debug!("syscall enter dlopen");
            match loader::sys_dlopen(x0, x1 as u32) {
                Ok(result) => result,
                Err(error) => error,
            }
        }
        HNX_SYS_DLCLOSE => {
            crate::debug!("syscall enter dlclose");
            match loader::sys_dlclose(x0) {
                Ok(result) => result,
                Err(error) => error,
            }
        }
        HNX_SYS_DLSYM => {
            crate::debug!("syscall enter dlsym");
            match loader::sys_dlsym(x0, x1) {
                Ok(result) => result,
                Err(error) => error,
            }
        }
        HNX_SYS_MMAP => {
            crate::debug!("syscall enter mmap");
            sys_mmap(x0, x1, x2, _x3, _x4, _x5)
        }
        HNX_SYS_MUNMAP => {
            crate::debug!("syscall enter munmap");
            sys_munmap(x0, x1)
        }
        HNX_SYS_MPROTECT => {
            crate::debug!("syscall enter mprotect");
            sys_mprotect(x0, x1, x2)
        }
        _ => -1,
    }
}

fn sys_write(fd: usize, buf: usize, len: usize) -> SysResult {
    crate::debug!("syscall sys_write enter fd={} len={}", fd, len);
    
    // Handle stdout/stderr
    if fd == 1 || fd == 2 {
        let len = core::cmp::min(len, 64);
        let mut tmp = [0u8; 1024];
        let mut written = 0usize;
        let mut off = 0usize;
        while off < len {
            let take = core::cmp::min(tmp.len(), len - off);
            let n = copy_from_user(buf + off, take, &mut tmp);
            if n == 0 {
                break;
            }
            let s = core::str::from_utf8(&tmp[..n]).unwrap_or("");
            crate::print!("{}", s);
            if n > 0 {
                crate::debug!("syscall write chunk n={} first=0x{:X}", n, tmp[0]);
            }
            written += n;
            off += n;
        }
        return written as SysResult;
    }
    
    // Handle file writes via VFS IPC
    let mut tbl = FD_TABLE.lock();
    if fd >= tbl.len() {
        return -1;
    }
    
    if let Some(ref mut e) = tbl[fd] {
        // Check if file is writable
        if !e.writable {
            crate::debug!("syscall write: file not opened for writing");
            return -1;  // EBADF - Bad file descriptor
        }
        
        // Build path with offset for VFS IPC
        let mut key = [0u8; 256];
        let mut pos = 0usize;
        let pn = core::cmp::min(e.name_len, 200);
        key[..pn].copy_from_slice(&e.name[..pn]);
        pos += pn;
        
        // Add space separator
        if pos < key.len() {
            key[pos] = b' ';
            pos += 1;
        }
        
        // Add offset as string
        let mut v = e.pos;
        let mut tmp = [0u8; 32];
        let mut di = 0usize;
        if v == 0 {
            tmp[di] = b'0';
            di += 1;
        } else {
            let mut buf = [0u8; 20];
            let mut i = 0usize;
            while v > 0 && i < buf.len() {
                buf[i] = b'0' + ((v % 10) as u8);
                v /= 10;
                i += 1;
            }
            while i > 0 {
                i -= 1;
                tmp[di] = buf[i];
                di += 1;
            }
        }
        let dn = core::cmp::min(tmp.len(), key.len().saturating_sub(pos));
        key[pos..pos + dn].copy_from_slice(&tmp[..dn]);
        pos += dn;
        
        // Copy data from user
        let mut kbuf = [0u8; 1024];
        let take = core::cmp::min(kbuf.len(), len);
        let n = copy_from_user(buf, take, &mut kbuf);
        if n == 0 {
            return -1;
        }
        
        // Call VFS write operation (op=8)
        let r = crate::kernel_core::ipc::call(crate::kernel_core::ipc::Msg {
            service: crate::kernel_core::ipc::SERVICE_VFS,
            op: 8,  // VFS_OP_WRITE
            p1: key.as_ptr(),
            l1: pos,
            p2: kbuf.as_ptr() as *mut u8,
            l2: n,
        });
        
        if r.code == 0 {
            // Update file offset
            e.pos = e.pos.saturating_add(r.len);
            return r.len as SysResult;
        } else {
            return -1;
        }
    }
    
    -1
}

fn sys_exit(_status: usize) -> SysResult {
    crate::debug!("syscall sys_exit status={}", _status);
    
    let current_pid = crate::kernel_core::scheduler::current_pid() as u32;
    
    // Set exit status and transition to zombie
    crate::process::set_exit_status(current_pid as usize, _status as i32);
    
    // Wake parent if waiting
    if let Some(parent_pid) = crate::process::get_parent_pid(current_pid as usize) {
        if parent_pid != 0 {
            let parent_state = crate::process::get_process_state(parent_pid as usize);
            if parent_state == Some(crate::process::ProcState::Waiting) {
                crate::info!("exit: waking parent pid={}", parent_pid);
                crate::process::wake_process(parent_pid as usize);
            }
        }
    }
    
    // TODO: Free resources (memory, file descriptors, etc.)
    // TODO: Orphan children (reparent to init)
    
    crate::kernel_core::scheduler::exit_current()
}

fn sys_getpid() -> SysResult {
    let r = crate::kernel_core::ipc::call(crate::kernel_core::ipc::Msg {
        service: crate::kernel_core::ipc::SERVICE_PROC,
        op: 1,
        p1: core::ptr::null(),
        l1: 0,
        p2: core::ptr::null_mut(),
        l2: 0,
    });
    if r.code == 0 {
        r.len as SysResult
    } else {
        -1
    }
}

// File descriptor open flags (using hnx-abi definitions)
use hnx_abi::{O_RDONLY, O_WRONLY, O_RDWR, O_CREAT, O_TRUNC, O_APPEND};

// File descriptor entry (using hnx-abi definition)
use hnx_abi::FdEntry;

static FD_TABLE: Mutex<[Option<FdEntry>; 32]> = Mutex::new([None; 32]);

fn sys_open(path_user: usize, flags: u32, mode: u32) -> SysResult {
    let mut name = [0u8; 128];
    let mut i = 0usize;
    while i < name.len() {
        let mut b = [0u8; 1];
        let n = copy_from_user(path_user + i, 1, &mut b);
        if n == 0 {
            break;
        }
        name[i] = b[0];
        if b[0] == 0 {
            break;
        }
        i += 1;
    }
    let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
    let p = &name[..end];
    
    // Check if we need to create the file
    let should_create = (flags & O_CREAT) != 0;
    
    // First, check if file exists
    let r = crate::kernel_core::ipc::call(crate::kernel_core::ipc::Msg {
        service: crate::kernel_core::ipc::SERVICE_VFS,
        op: 5,  // exists
        p1: p.as_ptr(),
        l1: p.len(),
        p2: core::ptr::null_mut(),
        l2: 0,
    });
    
    let file_exists = r.code == 1;
    
    // If file doesn't exist and O_CREAT is set, create it
    if !file_exists && should_create {
        let create_r = crate::kernel_core::ipc::call(crate::kernel_core::ipc::Msg {
            service: crate::kernel_core::ipc::SERVICE_VFS,
            op: 9,  // VFS_OP_CREATE
            p1: p.as_ptr(),
            l1: p.len(),
            p2: &mode as *const u32 as *mut u8,
            l2: core::mem::size_of::<u32>(),
        });
        
        if create_r.code != 0 {
            crate::debug!("syscall open: create failed");
            return -1;
        }
    } else if !file_exists {
        // File doesn't exist and O_CREAT not set
        return -1;  // ENOENT
    }
    
    // Get file size
    let sz = crate::kernel_core::ipc::call(crate::kernel_core::ipc::Msg {
        service: crate::kernel_core::ipc::SERVICE_VFS,
        op: 7,
        p1: p.as_ptr(),
        l1: p.len(),
        p2: core::ptr::null_mut(),
        l2: 0,
    });
    
    if sz.code != 0 {
        return -1;
    }
    
    // Allocate FD entry
    let mut tbl = FD_TABLE.lock();
    for fd in 3..tbl.len() {
        if tbl[fd].is_none() {
            let mut nm = [0u8; 128];
            let n = core::cmp::min(128, p.len());
            nm[..n].copy_from_slice(&p[..n]);
            
            // Determine if writable based on flags
            let writable = (flags & O_WRONLY) != 0 || (flags & O_RDWR) != 0;
            
            // If O_TRUNC is set and file is writable, truncate it
            if writable && (flags & O_TRUNC) != 0 {
                let _ = crate::kernel_core::ipc::call(crate::kernel_core::ipc::Msg {
                    service: crate::kernel_core::ipc::SERVICE_VFS,
                    op: 10,  // VFS_OP_TRUNCATE
                    p1: p.as_ptr(),
                    l1: p.len(),
                    p2: core::ptr::null_mut(),
                    l2: 0,
                });
            }
            
            tbl[fd] = Some(FdEntry {
                name: nm,
                name_len: n,
                size: if (flags & O_TRUNC) != 0 { 0 } else { sz.len },
                pos: 0,
                flags,
                writable,
            });
            return fd as SysResult;
        }
    }
    -2  // Too many open files
}

fn sys_read(fd: usize, buf_user: usize, len: usize) -> SysResult {
    let mut tbl = FD_TABLE.lock();
    if fd >= tbl.len() {
        return -1;
    }
    if let Some(ref mut e) = tbl[fd] {
        let remaining = e.size.saturating_sub(e.pos);
        let n = core::cmp::min(len, remaining);
        if n == 0 {
            return 0;
        }
        let mut key = [0u8; 256];
        let mut pos = 0usize;
        let pn = core::cmp::min(e.name_len, 200);
        key[..pn].copy_from_slice(&e.name[..pn]);
        pos += pn;
        if pos < key.len() {
            key[pos] = b' ';
            pos += 1;
        }
        let mut v = e.pos;
        let mut tmp = [0u8; 32];
        let mut di = 0usize;
        if v == 0 {
            tmp[di] = b'0';
            di += 1;
        } else {
            let mut buf = [0u8; 20];
            let mut i = 0usize;
            while v > 0 && i < buf.len() {
                buf[i] = b'0' + ((v % 10) as u8);
                v /= 10;
                i += 1;
            }
            while i > 0 {
                i -= 1;
                tmp[di] = buf[i];
                di += 1;
            }
        }
        let dn = core::cmp::min(tmp.len(), key.len().saturating_sub(pos));
        key[pos..pos + dn].copy_from_slice(&tmp[..dn]);
        pos += dn;
        let mut kbuf = [0u8; 1024];
        let take = core::cmp::min(kbuf.len(), n);
        let r = crate::kernel_core::ipc::call(crate::kernel_core::ipc::Msg {
            service: crate::kernel_core::ipc::SERVICE_VFS,
            op: 4,
            p1: key.as_ptr(),
            l1: pos,
            p2: kbuf.as_mut_ptr(),
            l2: take,
        });
        if r.code == 0 {
            let m = core::cmp::min(r.len, take);
            let written = copy_to_user(buf_user, &kbuf[..m]);
            e.pos = e.pos.saturating_add(m);
            return written as SysResult;
        } else {
            return -1;
        }
    }
    -1
}

fn sys_close(fd: usize) -> SysResult {
    let mut tbl = FD_TABLE.lock();
    if fd >= tbl.len() {
        return -1;
    }
    tbl[fd] = None;
    0
}

fn sys_ep_create() -> SysResult {
    // Create endpoint with full capabilities
    let capabilities = crate::kernel_core::ipc::EndpointCapabilities {
        read: true,
        write: true,
        admin: true,
    };
    
    match crate::kernel_core::ipc::endpoint_create(capabilities) {
        Ok(ep) => {
            let pid = crate::kernel_core::scheduler::current_pid() as usize;
            let h = caps_alloc_handle(pid, ep, RIGHT_SEND | RIGHT_RECV | RIGHT_MANAGE);
            if h == usize::MAX {
                -1
            } else {
                h as SysResult
            }
        }
        Err(_) => -1,
    }
}

fn sys_ep_send(handle: usize, op: usize, buf_user: usize, len: usize) -> SysResult {
    let mut k = [0u8; 256];
    let n = core::cmp::min(k.len(), len);
    let cn = copy_from_user(buf_user, n, &mut k);
    let src = crate::kernel_core::scheduler::current_pid() as u32;
    
    // Create message with priority
    let msg = crate::kernel_core::ipc::IpcMessage {
        src_pid: src,
        dst_epid: 0, // Will be set in endpoint_send_sync
        op: (op & 0xFFFF) as u16,
        priority: crate::kernel_core::ipc::Priority::Normal,
        data_len: cn,
        data: k,
        timestamp: crate::arch::timer::now_us(),
    };
    
    let pid = crate::kernel_core::scheduler::current_pid() as usize;
    if let Some(cap) = caps_lookup(pid, handle) {
        // Validate that the capability grants send rights
        if !validate_capability(cap.cap_id, cap.ep_id, rights::WRITE) {
            return -1;
        }
        
        if (cap.rights & RIGHT_SEND) == 0 {
            return -1;
        }
        
        // Send using new API
        match crate::kernel_core::ipc::endpoint_send_sync(cap.ep_id, msg) {
            Ok(_) => cn as SysResult,
            Err(_) => -1,
        }
    } else {
        -1
    }
}

fn sys_ep_recv(handle: usize, buf_user: usize, len: usize) -> SysResult {
    let pid = crate::kernel_core::scheduler::current_pid() as usize;
    if let Some(cap) = caps_lookup(pid, handle) {
        // Validate that the capability grants receive rights
        if !validate_capability(cap.cap_id, cap.ep_id, rights::READ) {
            return -1;
        }
        
        if (cap.rights & RIGHT_RECV) == 0 {
            return -1;
        }
        
        // Receive using new API
        match crate::kernel_core::ipc::endpoint_recv_sync(cap.ep_id, None) {
            Ok(msg) => {
                let n = core::cmp::min(len, msg.data_len);
                copy_to_user(buf_user, &msg.data[..n]) as SysResult
            }
            Err(_) => 0,
        }
    } else {
        -1
    }
}

fn sys_creat(path_user: usize, mode: u32) -> SysResult {
    crate::debug!("syscall sys_creat mode=0o{:o}", mode);
    
    // Read path from user space
    let mut name = [0u8; 128];
    let mut i = 0usize;
    while i < name.len() {
        let mut b = [0u8; 1];
        let n = copy_from_user(path_user + i, 1, &mut b);
        if n == 0 {
            break;
        }
        name[i] = b[0];
        if b[0] == 0 {
            break;
        }
        i += 1;
    }
    let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
    let p = &name[..end];
    
    // Call VFS create operation
    let r = crate::kernel_core::ipc::call(crate::kernel_core::ipc::Msg {
        service: crate::kernel_core::ipc::SERVICE_VFS,
        op: 9,  // VFS_OP_CREATE
        p1: p.as_ptr(),
        l1: p.len(),
        p2: &mode as *const u32 as *mut u8,
        l2: core::mem::size_of::<u32>(),
    });
    
    if r.code != 0 {
        return -1;
    }
    
    // Open the newly created file with write permissions
    sys_open(path_user, O_WRONLY, mode)
}

fn sys_unlink(path_user: usize) -> SysResult {
    crate::debug!("syscall sys_unlink");
    
    // Read path from user space
    let mut name = [0u8; 128];
    let mut i = 0usize;
    while i < name.len() {
        let mut b = [0u8; 1];
        let n = copy_from_user(path_user + i, 1, &mut b);
        if n == 0 {
            break;
        }
        name[i] = b[0];
        if b[0] == 0 {
            break;
        }
        i += 1;
    }
    let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
    let p = &name[..end];
    
    // Call VFS delete operation
    let r = crate::kernel_core::ipc::call(crate::kernel_core::ipc::Msg {
        service: crate::kernel_core::ipc::SERVICE_VFS,
        op: 11,  // VFS_OP_DELETE
        p1: p.as_ptr(),
        l1: p.len(),
        p2: core::ptr::null_mut(),
        l2: 0,
    });
    
    if r.code == 0 {
        0
    } else {
        -1
    }
}

fn sys_mkdir(path_user: usize, mode: u32) -> SysResult {
    crate::debug!("syscall sys_mkdir mode=0o{:o}", mode);
    
    // Read path from user space
    let mut name = [0u8; 128];
    let mut i = 0usize;
    while i < name.len() {
        let mut b = [0u8; 1];
        let n = copy_from_user(path_user + i, 1, &mut b);
        if n == 0 {
            break;
        }
        name[i] = b[0];
        if b[0] == 0 {
            break;
        }
        i += 1;
    }
    let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
    let p = &name[..end];
    
    // Call VFS mkdir operation
    let r = crate::kernel_core::ipc::call(crate::kernel_core::ipc::Msg {
        service: crate::kernel_core::ipc::SERVICE_VFS,
        op: 12,  // VFS_OP_MKDIR
        p1: p.as_ptr(),
        l1: p.len(),
        p2: &mode as *const u32 as *mut u8,
        l2: core::mem::size_of::<u32>(),
    });
    
    if r.code == 0 {
        0
    } else {
        -1
    }
}

fn sys_rmdir(path_user: usize) -> SysResult {
    crate::debug!("syscall sys_rmdir");
    
    // Read path from user space
    let mut name = [0u8; 128];
    let mut i = 0usize;
    while i < name.len() {
        let mut b = [0u8; 1];
        let n = copy_from_user(path_user + i, 1, &mut b);
        if n == 0 {
            break;
        }
        name[i] = b[0];
        if b[0] == 0 {
            break;
        }
        i += 1;
    }
    let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
    let p = &name[..end];
    
    // Call VFS rmdir operation
    let r = crate::kernel_core::ipc::call(crate::kernel_core::ipc::Msg {
        service: crate::kernel_core::ipc::SERVICE_VFS,
        op: 13,  // VFS_OP_RMDIR
        p1: p.as_ptr(),
        l1: p.len(),
        p2: core::ptr::null_mut(),
        l2: 0,
    });
    
    if r.code == 0 {
        0
    } else {
        -1
    }
}

fn sys_mmap(addr: usize, length: usize, prot: usize, flags: usize, fd: usize, offset: usize) -> SysResult {
    crate::debug!("syscall sys_mmap addr=0x{:x} length={} prot={} flags={} fd={} offset={}", 
                  addr, length, prot, flags, fd, offset);
    
    // Validate parameters
    if length == 0 {
        return -1; // EINVAL
    }
    
    // Align length to page size
    let page_size = 4096; // Assuming 4KB pages
    let aligned_length = (length + page_size - 1) & !(page_size - 1);
    
    // Convert prot flags to MemoryMapFlags
    let readable = (prot & 0x1) != 0; // PROT_READ
    let writable = (prot & 0x2) != 0; // PROT_WRITE
    let executable = (prot & 0x4) != 0; // PROT_EXEC
    
    // Determine mapping type based on flags
    let map_type = if (flags & 0x20) != 0 { // MAP_ANONYMOUS
        crate::memory::mmap_manager::MemoryMapType::Anonymous
    } else {
        crate::memory::mmap_manager::MemoryMapType::File
    };
    
    // Determine the address to use
    let effective_addr = if addr == 0 {
        // Need to find a suitable address - for now we'll use a fixed range
        // In a real implementation, we'd search for a free region
        0x10000000 // Start of user memory mapping area
    } else {
        // Validate address alignment
        if addr % page_size != 0 {
            return -1; // EINVAL - address must be page-aligned
        }
        addr
    };
    
    // Create memory mapping flags
    let map_flags = crate::memory::mmap_manager::MemoryMapFlags::new(
        readable, 
        writable, 
        executable, 
        true, // user_accessible
        true  // cacheable
    );
    
    // Create memory mapping entry
    let mut entry = crate::memory::mmap_manager::MemoryMapEntry::new(
        effective_addr, 
        aligned_length, 
        map_type, 
        map_flags
    );
    
    // Set file descriptor data if this is a file mapping
    if map_type == crate::memory::mmap_manager::MemoryMapType::File {
        entry.data = Some(fd as u64);
    }
    
    // Add to memory map manager
    match crate::memory::mmap_manager::add_memory_map(entry) {
        Ok(()) => effective_addr as SysResult,
        Err(()) => -1 // Failed to add mapping (likely due to overlap)
    }
}

fn sys_munmap(addr: usize, length: usize) -> SysResult {
    crate::debug!("syscall sys_munmap addr=0x{:x} length={}", addr, length);
    
    // Validate parameters
    if length == 0 {
        return -1; // EINVAL
    }
    
    // Align length to page size
    let page_size = 4096; // Assuming 4KB pages
    let aligned_length = (length + page_size - 1) & !(page_size - 1);
    
    // Validate address alignment
    if addr % page_size != 0 {
        return -1; // EINVAL - address must be page-aligned
    }
    
    // Check if there are any mappings in the specified range
    let range = addr..(addr + aligned_length);
    {
        let manager = crate::memory::mmap_manager::MEMORY_MAP_MANAGER.lock();
        let overlapping_entries = manager.find_overlapping_entries(range.clone());
        if overlapping_entries.is_empty() {
            // No mappings found in the specified range, but this is not an error according to POSIX
            return 0;
        }
    } // Release the lock
    
    // Remove memory mappings in the specified range
    let removed_count = crate::memory::mmap_manager::remove_memory_maps_in_range(range);
    
    // Return success (0)
    0
}

fn sys_mprotect(addr: usize, length: usize, prot: usize) -> SysResult {
    crate::debug!("syscall sys_mprotect addr=0x{:x} length={} prot={}", addr, length, prot);
    
    // Validate parameters
    if length == 0 {
        return -1; // EINVAL
    }
    
    // Validate address alignment
    let page_size = 4096; // Assuming 4KB pages
    if addr % page_size != 0 {
        return -1; // EINVAL - address must be page-aligned
    }
    
    // Align length to page size
    let aligned_length = (length + page_size - 1) & !(page_size - 1);
    
    // Convert prot flags to MemoryMapFlags
    let readable = (prot & 0x1) != 0; // PROT_READ
    let writable = (prot & 0x2) != 0; // PROT_WRITE
    let executable = (prot & 0x4) != 0; // PROT_EXEC
    
    // Find memory mapping entries that overlap with the specified range
    let range = addr..(addr + aligned_length);
    
    // Lock the memory map manager for the entire operation to ensure atomicity
    let mut manager = crate::memory::mmap_manager::MEMORY_MAP_MANAGER.lock();
    
    // Check if there are overlapping entries
    let overlapping_entries = manager.find_overlapping_entries(range.clone());
    if overlapping_entries.is_empty() {
        return -1; // ENOMEM - No mapping found in the specified range
    }
    
    // Drop the overlapping_entries reference to avoid borrowing conflicts
    drop(overlapping_entries);
    
    // Update the flags for all overlapping entries, handling partial overlaps
    let new_flags = crate::memory::mmap_manager::MemoryMapFlags::new(
        readable, 
        writable, 
        executable, 
        true, // user_accessible - assuming user mappings
        true  // cacheable
    );
    
    match manager.update_flags_with_partial_overlap(range, new_flags) {
        Ok(()) => 0,   // Success
        Err(()) => -1  // Failed to update mapping
    }
}
