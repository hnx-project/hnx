// ARMv8-A 64位架构实现
//
// 实现所有架构抽象特征。
// 使用tock-registers进行寄存器访问。
// 使用aarch64-cpu进行CPU指令封装。
// 使用smccc进行安全监控调用。

#![allow(unused_imports)]

pub mod boot;
pub mod cpu;
pub mod mmu;
pub mod interrupt;
pub mod timer;
pub mod exception;
pub mod smp;
pub mod psci;
pub mod registers;

use crate::arch::traits::*;
use crate::arch::ArchResult;
use crate::arch::traits::exception::{ExceptionVector, BreakpointKind, BreakpointHandle, ExceptionAction, ExceptionHandler};
use crate::arch::traits::boot::MemoryRegion;
use crate::arch::traits::timer::{TimerCallback, TimerHandle};
use crate::arch::traits::mmu::{MappingFlags, MappingInfo};
use crate::arch::traits::interrupt::{InterruptHandler, IrqState};

// 重新导出具体实现类型
pub use boot::Aarch64Boot;
pub use cpu::Aarch64Cpu;
pub use mmu::Aarch64Mmu;
pub use interrupt::Aarch64Interrupt;
pub use timer::Aarch64Timer;
pub use exception::Aarch64Exception;
pub use boot::{Aarch64BootInfo, Aarch64DeviceTree};
pub use cpu::{Aarch64Context, Aarch64CpuInfo};
pub use exception::{Aarch64ExceptionContext, Aarch64ExceptionVectorTable};
pub use interrupt::{Aarch64Gic, Aarch64IrqDescriptor, Aarch64IrqNumber};
pub use mmu::{Aarch64AddressSpace, Aarch64MemoryRegion, Aarch64PageTable, Aarch64PageTableEntry};
pub use timer::{Aarch64ArchTimer, Aarch64GenericTimer};

/// ARMv8-A架构实现
pub struct Aarch64Arch;

impl CpuArch for Aarch64Arch {
    type CpuInfo = Aarch64CpuInfo;
    type Context = Aarch64Context;
    
    fn init() -> ArchResult<()> { cpu::init() }
    fn current_cpu() -> Self::CpuInfo { cpu::current_cpu() }
    fn cpu_count() -> usize { cpu::cpu_count() }
    fn start_secondary(cpu_id: usize, entry_point: usize) -> ArchResult<()> { cpu::start_secondary(cpu_id, entry_point) }
    fn data_synchronization_barrier() { cpu::data_synchronization_barrier() }
    fn instruction_synchronization_barrier() { cpu::instruction_synchronization_barrier() }
    fn memory_barrier() { cpu::memory_barrier() }
    fn wait_for_interrupt() { cpu::wait_for_interrupt() }
    fn disable_interrupts() { cpu::disable_interrupts() }
    fn enable_interrupts() { cpu::enable_interrupts() }
    fn save_and_disable_interrupts() -> bool { cpu::save_and_disable_interrupts() }
    fn restore_interrupts(previous_state: bool) { cpu::restore_interrupts(previous_state) }
    fn current_privilege_level() -> u8 { cpu::current_privilege_level() }
    fn set_stack_pointer(sp: usize) { cpu::set_stack_pointer(sp) }
    fn read_timestamp_counter() -> u64 { cpu::read_timestamp_counter() }
}

impl MmuArch for Aarch64Arch {
    type PageTable = Aarch64PageTable;
    type PageTableEntry = Aarch64PageTableEntry;
    type AddressSpace = Aarch64AddressSpace;
    type MemoryRegion = Aarch64MemoryRegion;
    
    fn init() -> ArchResult<()> { mmu::init() }
    fn create_page_table() -> ArchResult<Self::PageTable> { mmu::create_page_table() }
    fn current_page_table() -> Self::PageTable { mmu::current_page_table() }
    fn switch_page_table(table: &Self::PageTable) { mmu::switch_page_table(table) }
    fn map(table: &mut Self::PageTable, vaddr: usize, paddr: usize, size: usize, flags: MappingFlags) -> ArchResult<()> { mmu::map(table, vaddr, paddr, size, flags) }
    fn unmap(table: &mut Self::PageTable, vaddr: usize, size: usize) -> ArchResult<()> { mmu::unmap(table, vaddr, size) }
    fn query(table: &Self::PageTable, vaddr: usize) -> ArchResult<MappingInfo> { mmu::query(table, vaddr) }
    fn tlb_flush(vaddr: Option<usize>, asid: Option<u16>) { mmu::tlb_flush(vaddr, asid) }
    fn cache_invalidate(vaddr: usize, size: usize) { mmu::cache_invalidate(vaddr, size) }
    fn page_size() -> usize { mmu::page_size() }
    fn supported_huge_page_sizes() -> &'static [usize] { mmu::supported_huge_page_sizes() }
    fn allocate_physical_page() -> ArchResult<usize> { mmu::allocate_physical_page() }
    fn free_physical_page(paddr: usize) -> ArchResult<()> { mmu::free_physical_page(paddr) }
}

impl InterruptArch for Aarch64Arch {
    type Controller = Aarch64Gic;
    type Descriptor = Aarch64IrqDescriptor;
    type IrqNumber = Aarch64IrqNumber;
    
    fn init() -> ArchResult<()> { interrupt::init() }
    fn controller() -> &'static Self::Controller { interrupt::controller() }
    fn register_handler(irq: Self::IrqNumber, handler: InterruptHandler, data: usize) -> ArchResult<()> { interrupt::register_handler(irq, handler, data) }
    fn unregister_handler(irq: Self::IrqNumber) -> ArchResult<()> { interrupt::unregister_handler(irq) }
    fn enable_irq(irq: Self::IrqNumber) -> ArchResult<()> { interrupt::enable_irq(irq) }
    fn disable_irq(irq: Self::IrqNumber) -> ArchResult<()> { interrupt::disable_irq(irq) }
    fn get_irq_state(irq: Self::IrqNumber) -> IrqState { interrupt::get_irq_state(irq) }
    fn send_eoi(irq: Self::IrqNumber) { interrupt::send_eoi(irq) }
    fn set_priority(irq: Self::IrqNumber, priority: u8) -> ArchResult<()> { interrupt::set_priority(irq, priority) }
    fn set_affinity(irq: Self::IrqNumber, cpu_mask: u32) -> ArchResult<()> { interrupt::set_affinity(irq, cpu_mask) }
    fn trigger_software_irq(irq: Self::IrqNumber, cpu_mask: u32) -> ArchResult<()> { interrupt::trigger_software_irq(irq, cpu_mask) }
}

impl TimerArch for Aarch64Arch {
    type Timer = Aarch64GenericTimer;
    type ClockSource = Aarch64ArchTimer;
    
    fn init() -> ArchResult<()> { timer::init() }
    fn system_time() -> u64 { timer::system_time() }
    fn monotonic_time() -> u64 { timer::monotonic_time() }
    fn boot_time() -> u64 { timer::boot_time() }
    fn high_precision_timer() -> &'static Self::Timer { timer::high_precision_timer() }
    fn system_timer() -> &'static Self::Timer { timer::system_timer() }
    fn register_callback(callback: TimerCallback, data: usize) -> ArchResult<TimerHandle> { timer::register_callback(callback, data) }
    fn unregister_callback(handle: TimerHandle) -> ArchResult<()> { timer::unregister_callback(handle) }
    fn set_oneshot(timer: &Self::Timer, nanoseconds: u64, callback: Option<TimerCallback>, data: usize) -> ArchResult<()> { timer::set_oneshot(timer, nanoseconds, callback, data) }
    fn set_periodic(timer: &Self::Timer, period_ns: u64, callback: Option<TimerCallback>, data: usize) -> ArchResult<()> { timer::set_periodic(timer, period_ns, callback, data) }
    fn cancel(timer: &Self::Timer) -> ArchResult<()> { timer::cancel(timer) }
    fn timer_frequency(timer: &Self::Timer) -> u64 { timer::timer_frequency(timer) }
    fn clock_source() -> &'static Self::ClockSource { timer::clock_source() }
    fn calibrate() -> ArchResult<()> { timer::calibrate() }
}

impl ExceptionArch for Aarch64Arch {
    type Context = Aarch64ExceptionContext;
    type VectorTable = Aarch64ExceptionVectorTable;
    
    fn init() -> ArchResult<()> { exception::init() }
    fn set_vector_table(table: &Self::VectorTable) { exception::set_vector_table(table) }
    unsafe fn current_context() -> Self::Context { exception::current_context() }
    unsafe fn restore_context(context: Self::Context) { exception::restore_context(context) }
    fn handle_exception(context: &mut Self::Context) -> ExceptionAction { exception::handle_exception(context) }
    fn register_handler(vector: ExceptionVector, handler: ExceptionHandler, data: usize) -> ArchResult<()> { exception::register_handler(vector, handler, data) }
    fn unregister_handler(vector: ExceptionVector) -> ArchResult<()> { exception::unregister_handler(vector) }
    fn trigger_software_exception(vector: ExceptionVector, code: u32) -> ArchResult<()> { exception::trigger_software_exception(vector, code) }
    fn set_breakpoint(address: usize, kind: BreakpointKind) -> ArchResult<BreakpointHandle> { exception::set_breakpoint(address, kind) }
    fn clear_breakpoint(handle: BreakpointHandle) -> ArchResult<()> { exception::clear_breakpoint(handle) }
    fn enable_single_step(context: &mut Self::Context) { exception::enable_single_step(context) }
    fn disable_single_step(context: &mut Self::Context) { exception::disable_single_step(context) }
}

impl BootArch for Aarch64Arch {
    type BootInfo = Aarch64BootInfo;
    type DeviceTree = Aarch64DeviceTree;
    
    unsafe fn early_init() { boot::early_init() }
    fn init() -> ArchResult<Self::BootInfo> { boot::init() }
    fn boot_info() -> &'static Self::BootInfo { boot::boot_info() }
    fn init_memory(info: &Self::BootInfo) -> ArchResult<()> { boot::init_memory(info) }
    fn init_console() -> ArchResult<()> { boot::init_console() }
    fn init_smp() -> ArchResult<()> { boot::init_smp() }
    fn boot_secondary_cpus() -> ArchResult<()> { boot::boot_secondary_cpus() }
    fn platform_init() -> ArchResult<()> { boot::platform_init() }
    fn device_tree() -> Option<&'static Self::DeviceTree> { boot::device_tree() }
    fn command_line() -> &'static str { boot::command_line() }
    fn memory_map() -> &'static [MemoryRegion] { boot::memory_map() }
    fn boot_timestamp() -> u64 { boot::boot_timestamp() }
    fn cleanup_boot_resources() -> ArchResult<()> { boot::cleanup_boot_resources() }
}
