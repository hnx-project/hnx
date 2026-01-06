use crate::arch::boot::get_boot_info;
use crate::arch::common::mmu::MmuFlags;
use crate::console;
use crate::fs::elf;
use crate::core::scheduler;
use crate::memory::virt::{create_user_l1, map, map_in_pt, KERNEL_BASE};
use crate::process::task::Task;
use crate::{error, info};
use crate::{fs, print};

extern "C" {
    fn user_store_ok_svc1() -> !;
}

// static EMBED_ELF: &[u8] = include_bytes!("../");

// 执行内置的 hello.elf 程序
pub fn sys_execve_builtin() -> isize {
    if let Some(pt_base) = create_user_l1() {
        let user_sp = 0x0010_0000usize;
        if let Some(code_pa) = crate::memory::physical::alloc_pages(1) {
            if let Some(data_pa) = crate::memory::physical::alloc_pages(1) {
                let code_va = 0x0000_1000usize;
                let data_va = 0x0000_2000usize;
                let cf = MmuFlags::READ
                    .combine(MmuFlags::EXECUTE)
                    .combine(MmuFlags::USER);
                let df = MmuFlags::READ
                    .combine(MmuFlags::WRITE)
                    .combine(MmuFlags::USER);
                map_in_pt(pt_base, code_va, code_pa, cf);
                map_in_pt(pt_base, data_va, data_pa, df);
                map_in_pt(pt_base, user_sp - 0x1000, data_pa, df);
                info!(
                    "builtin: mapped code_va=0x{:016X} data_va=0x{:016X} sp_page=0x{:016X}",
                    code_va,
                    data_va,
                    user_sp - 0x1000
                );
                unsafe {
                    let code_low = code_pa;
                    let data_low = data_pa;
                    info!(
                        "builtin: lowva code=0x{:016X} data=0x{:016X}",
                        code_low, data_low
                    );
                    let src = user_store_ok_svc1 as *const u8;
                    for i in 0..24 {
                        let b = core::ptr::read(src.add(i));
                        core::ptr::write((code_low as *mut u8).add(i), b);
                    }
                    let w0 = core::ptr::read(code_low as *const u32);
                    let w1 = core::ptr::read((code_low as *const u32).add(1));
                    let w2 = core::ptr::read((code_low as *const u32).add(2));
                    info!(
                        "builtin: code words w0=0x{:08X} w1=0x{:08X} w2=0x{:08X}",
                        w0, w1, w2
                    );
                    let s = b"OK\n";
                    for i in 0..s.len() {
                        core::ptr::write((data_low as *mut u8).add(i), s[i]);
                    }
                    info!("builtin: code+data written");
                    crate::arch::mmu::clean_dcache_range(code_low, 4096);
                    crate::arch::mmu::clean_dcache_range(data_low, 4096);
                }
                info!("builtin: dcache cleaned");
                crate::arch::mmu::flush_icache_all();
                info!("builtin: icache flushed");
                let entry = code_va;
                info!("svc selftest entry=0x{:016X} buf=0x{:016X}", entry, data_va);
                let mut task =
                    Task::new_kernel(unsafe { core::mem::transmute::<usize, fn() -> !>(entry) });
                unsafe {
                    task.ttbr0_base = pt_base;
                    task.context.sp = user_sp;
                    task.entry_point = entry;
                }
                let fd = 1usize;
                let len = 3usize;
                info!("builtin: starting arch_do_exec");
                scheduler::run_task_with_args(task, fd, data_va, len, 1)
            } else {
                error!("alloc data page failed");
                return -1;
            }
        } else {
            error!("alloc code page failed");
            return -1;
        }
    }
    error!("create user L1 failed");
    -1
}

// 执行 initrd 中的 ELF 程序
// pub fn sys_execve_elf() -> isize {
//     info!("exec: start sys_execve_elf");
//     let boot = get_boot_info();
//     if boot.initrd_size == 0 {
//         info!("exec: initrd size=0, using scan window");
//     }
//     let bytes = fs::initrd_bytes();
//     info!("exec: initrd slice len=0x{:X}", bytes.len());
//     crate::console::write_raw("initrd slice prepared\n");
//     crate::console::write_raw("check first4 begin\n");
//     if bytes.len() >= 16 {
//         let b0 = unsafe { *bytes.get_unchecked(0) };
//         let b1 = unsafe { *bytes.get_unchecked(1) };
//         let b2 = unsafe { *bytes.get_unchecked(2) };
//         let b3 = unsafe { *bytes.get_unchecked(3) };
//         let mut msg = [0u8; 64];
//         let mut p = 0usize;
//         let pre = b"first4 ";
//         for i in 0..pre.len() {
//             msg[p] = pre[i];
//             p += 1;
//         }
//         const H: &[u8; 16] = b"0123456789ABCDEF";
//         let bs = [b0, b1, b2, b3];
//         for (i, &v) in bs.iter().enumerate() {
//             msg[p] = H[(v >> 4) as usize];
//             p += 1;
//             msg[p] = H[(v & 0xF) as usize];
//             p += 1;
//             if i != 3 {
//                 msg[p] = b' ';
//                 p += 1;
//             }
//         }
//         msg[p] = b'\n';
//         p += 1;
//         let s = unsafe { core::str::from_utf8_unchecked(&msg[..p]) };
//         crate::console::write_raw(s);
//         // also dump next 12 bytes
//         let mut msg2 = [0u8; 96];
//         let mut q = 0usize;
//         let pre2 = b"next12 ";
//         for i in 0..pre2.len() {
//             msg2[q] = pre2[i];
//             q += 1;
//         }
//         let mut i = 4;
//         while i < 16 {
//             let v = unsafe { *bytes.get_unchecked(i) };
//             msg2[q] = H[(v >> 4) as usize];
//             q += 1;
//             msg2[q] = H[(v & 0xF) as usize];
//             q += 1;
//             if i != 15 {
//                 msg2[q] = b' ';
//                 q += 1;
//             }
//             i += 1;
//         }
//         msg2[q] = b'\n';
//         q += 1;
//         let s2 = unsafe { core::str::from_utf8_unchecked(&msg2[..q]) };
//         crate::console::write_raw(s2);
//     } else {
//         crate::console::write_raw("initrd too small for first16\n");
//     }
//     crate::console::write_raw("first4 cmp branch\n");
//     if bytes.len() >= 4 && bytes[0..4] == [0x7F, b'E', b'L', b'F'] {
//         crate::console::write_raw("early raw ELF detected, loading\n");
//         match elf::load(bytes) {
//             Ok(uprog) => {
//                 crate::console::write_raw("early elf load ok\n");
//                 let mut task = Task::new_kernel(unsafe {
//                     core::mem::transmute::<usize, fn() -> !>(uprog.entry)
//                 });
//                 unsafe {
//                     task.ttbr0_base = uprog.pt_base;
//                     task.context.sp = uprog.user_sp;
//                     task.entry_point = uprog.entry;
//                 }
//                 info!("exec: scheduling user task (early raw)");
//                 crate::console::write_raw("switching to EL0 (early raw)\n");
//                 scheduler::run_task(task);
//                 unreachable!();
//             }
//             Err(_) => {
//                 error!("exec: early elf load failed");
//                 crate::console::write_raw("early load failed, try embedded\n");
//                 if let Some(fb) = elf::find_elf(EMBED_ELF) {
//                     match elf::load(fb) {
//                         Ok(uprog) => {
//                             crate::console::write_raw("embedded elf load ok\n");
//                             let mut task = Task::new_kernel(unsafe {
//                                 core::mem::transmute::<usize, fn() -> !>(uprog.entry)
//                             });
//                             unsafe {
//                                 task.ttbr0_base = uprog.pt_base;
//                                 task.context.sp = uprog.user_sp;
//                                 task.entry_point = uprog.entry;
//                             }
//                             info!("exec: scheduling user task (embedded)");
//                             crate::console::write_raw("switching to EL0 (embedded)\n");
//                             scheduler::run_task(task);
//                             unreachable!();
//                         }
//                         Err(_) => {
//                             error!("exec: embedded elf load failed");
//                         }
//                     }
//                 }
//             }
//         }
//     }
//     if bytes.len() >= 4 {
//         crate::console::write_raw("initrd first4 checked\n");
//     }
//     if bytes.len() >= 4 {
//         let b0 = bytes[0];
//         let b1 = bytes[1];
//         let b2 = bytes[2];
//         let b3 = bytes[3];
//         let mut msg = [0u8; 64];
//         let mut p = 0usize;
//         let pre = b"first4 ";
//         for i in 0..pre.len() {
//             msg[p] = pre[i];
//             p += 1;
//         }
//         const H: &[u8; 16] = b"0123456789ABCDEF";
//         let bs = [b0, b1, b2, b3];
//         for (i, &v) in bs.iter().enumerate() {
//             msg[p] = H[(v >> 4) as usize];
//             p += 1;
//             msg[p] = H[(v & 0xF) as usize];
//             p += 1;
//             if i != 3 {
//                 msg[p] = b' ';
//                 p += 1;
//             }
//         }
//         msg[p] = b'\n';
//         p += 1;
//         let s = unsafe { core::str::from_utf8_unchecked(&msg[..p]) };
//         crate::console::write_raw(s);
//     }

//     crate::console::write_raw("vfs find_file /bin/hello.elf\n");
//     let raw_elf = bytes.len() >= 4 && bytes[0..4] == [0x7F, b'E', b'L', b'F'];
//     let mut found = if raw_elf {
//         crate::console::write_raw("detected raw ELF at initrd_start\n");
//         Some((bytes.as_ptr() as usize, bytes.len()))
//     } else {
//         crate::fs::vfs::find_file(bytes, "/bin/hello.elf")
//     };
//     crate::console::write_raw(if found.is_some() {
//         "vfs found /bin/hello.elf\n"
//     } else {
//         "vfs not found /bin/hello.elf\n"
//     });
//     if found.is_none() && bytes.len() >= 4 && bytes[0..4] == [0x7F, b'E', b'L', b'F'] {
//         crate::console::write_raw("raw ELF detected at initrd_start\n");
//     }
//     if let Some((start, size)) = found {
//         crate::console::write_raw("path load begin\n");
//         info!(
//             "exec: vfs find_file(/bin/hello.elf) ok start=0x{:X} size=0x{:X}",
//             start, size
//         );
//         info!("exec: found /bin/hello.elf in initrd (size=0x{:X})", size);
//         let elf_bytes = unsafe { core::slice::from_raw_parts(start as *const u8, size) };
//         crate::console::write_raw("elf(path) load...\n");
//         match elf::load(elf_bytes) {
//             Ok(uprog) => {
//                 crate::console::write_raw("elf(path) load ok\n");
//                 info!(
//                     "exec: elf(path) load ok entry=0x{:016X} sp=0x{:016X} ttbr0=0x{:016X}",
//                     uprog.entry, uprog.user_sp, uprog.pt_base
//                 );
//                 let mut task = Task::new_kernel(unsafe {
//                     core::mem::transmute::<usize, fn() -> !>(uprog.entry)
//                 });
//                 unsafe {
//                     task.ttbr0_base = uprog.pt_base;
//                     task.context.sp = uprog.user_sp;
//                     task.entry_point = uprog.entry;
//                 }
//                 info!("exec: scheduling user task (path)");
//                 crate::console::write_raw("switching to EL0 (path)\n");
//                 return scheduler::run_task(task);
//             }
//             Err(_) => {
//                 error!("exec: elf(path) load failed");
//             }
//         }
//     } else {
//         crate::console::write_raw("path not found, try ohlink\n");
//         info!("exec: /bin/hello.elf not present, trying /bin/hello.exe.ohlink");
//         if let Some(oh) = crate::fs::ohlink::find_ohlink(bytes, "/bin/hello.exe.ohlink") {
//             match crate::fs::ohlink::load(oh) {
//                 Ok(uprog) => {
//                     crate::console::write_raw("ohlink(path) load ok\n");
//                     info!(
//                         "exec: ohlink(path) load ok entry=0x{:016X} sp=0x{:016X} ttbr0=0x{:016X}",
//                         uprog.entry, uprog.user_sp, uprog.pt_base
//                     );
//                     let mut task = Task::new_kernel(unsafe {
//                         core::mem::transmute::<usize, fn() -> !>(uprog.entry)
//                     });
//                     unsafe {
//                         task.ttbr0_base = uprog.pt_base;
//                         task.context.sp = uprog.user_sp;
//                         task.entry_point = uprog.entry;
//                     }
//                     info!("exec: scheduling user task (ohlink path)");
//                     crate::console::write_raw("switching to EL0 (ohlink)\n");
//                     return scheduler::run_task(task);
//                 }
//                 Err(_) => {
//                     error!("exec: ohlink(path) load failed");
//                 }
//             }
//         }
//         info!("exec: /bin paths not present, fallback to magic scan");
//         crate::console::write_raw("start magic scan\n");
//     }
//     let src_primary = bytes;
//     let src_fallback = EMBED_ELF;
//     info!("exec: searching initrd for ELF magic");
//     crate::console::write_raw("scan initrd for ELF\n");
//     let elf_candidate = if let Some(b) = elf::find_elf(src_primary) {
//         info!("exec: ELF found in initrd region");
//         crate::console::write_raw("ELF found in initrd\n");
//         Some(b)
//     } else {
//         info!("exec: no ELF in initrd, trying embedded");
//         crate::console::write_raw("no ELF in initrd, try embedded\n");
//         if src_fallback.len() >= 16 {
//             let mut b: [u8; 16] = [0; 16];
//             for (i, v) in b.iter_mut().enumerate().take(16) {
//                 *v = unsafe { *src_fallback.get_unchecked(i) };
//             }
//             info!(
//                 "exec: embedded[0..16] = {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
//                 b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
//             );
//         }
//         elf::find_elf(src_fallback)
//     };
//     if let Some(elf_bytes) = elf_candidate {
//         info!("exec: elf header located");
//         crate::console::write_raw("elf header located, loading\n");
//         match elf::load(elf_bytes) {
//             Ok(uprog) => {
//                 info!(
//                     "exec: elf load ok entry=0x{:016X} sp=0x{:016X} ttbr0=0x{:016X}",
//                     uprog.entry, uprog.user_sp, uprog.pt_base
//                 );
//                 crate::console::write_raw("elf load ok\n");
//                 let mut task = Task::new_kernel(unsafe {
//                     core::mem::transmute::<usize, fn() -> !>(uprog.entry)
//                 });
//                 unsafe {
//                     task.ttbr0_base = uprog.pt_base;
//                     task.context.sp = uprog.user_sp;
//                     task.entry_point = uprog.entry;
//                 }
//                 info!("exec: scheduling user task");
//                 crate::console::write_raw("switching to EL0\n");
//                 scheduler::run_task(task)
//             }
//             Err(_) => {
//                 error!("exec: elf load failed");
//                 -1
//             }
//         }
//     } else {
//         error!("exec: no elf in initrd or embedded");
//         -1
//     }
// }

pub fn sys_execve_start() -> isize {
    print!("exec: try builtin first\n");
    let br = sys_execve_builtin();
    if br >= 0 {
        return br;
    }
    print!("exec: builtin failed, try initrd elf\n");
    // let r = sys_execve_elf();
    let r = -1;
    if r < 0 {
        info!("exec: elf failed, falling back to builtin");
        sys_execve_builtin()
    } else {
        r
    }
}