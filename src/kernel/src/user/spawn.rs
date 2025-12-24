use crate::process::{create_process, set_process_state, ProcState, Task};
use crate::core::scheduler;
use crate::{error, info};
use super::loader::{create_simple_user_program, UserProgram};

extern "C" {
    fn user_store_ok_svc1() -> !;
}

pub fn sys_process_create(code_ptr: usize, code_len: usize) -> isize {
    info!("user/spawn: creating user process from code_ptr=0x{:X} len={}", code_ptr, code_len);
    
    let code_bytes = if code_ptr == 0 {
        unsafe {
            let src = user_store_ok_svc1 as *const u8;
            core::slice::from_raw_parts(src, 24)
        }
    } else {
        unsafe {
            core::slice::from_raw_parts(code_ptr as *const u8, code_len.min(4096))
        }
    };
    
    let uprog = match create_simple_user_program(code_bytes) {
        Ok(p) => p,
        Err(_) => {
            error!("user/spawn: failed to create user program");
            return -1;
        }
    };
    
    let pid = match create_process(128) {
        Some(p) => p,
        None => {
            error!("user/spawn: failed to create PCB");
            return -1;
        }
    };
    
    let mut task = Task::new_kernel(unsafe {
        core::mem::transmute::<usize, fn() -> !>(uprog.entry)
    });
    
    unsafe {
        task.ttbr0_base = uprog.pt_base;
        task.context.sp = uprog.user_sp;
        task.entry_point = uprog.entry;
    }
    
    set_process_state(pid as usize, ProcState::Ready);
    
    info!("user/spawn: created process pid={} entry=0x{:X}", pid, uprog.entry);
    
    scheduler::run_task(task)
}