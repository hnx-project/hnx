// ARMv8-A mmu模块实现
//
// 要求实现：
// - 定义具体类型：
//   Aarch64mmu 等
// - 实现架构抽象特征对应的具体方法
// - 使用tock-registers定义寄存器
// - 使用aarch64-cpu访问CPU功能
// - 遵循"一切皆对象"原则
//
// 主要API：
// - init(): 初始化本模块
// - 实现对应trait的所有方法
// - 提供对象构造和销毁接口
//
// 对象类型：
// - Aarch64mmu: 主对象类型
// - Aarch64mmuInfo: 信息对象
// - Aarch64mmuConfig: 配置对象
// - Aarch64mmuState: 状态对象
//
// 关键功能：
// - 页表描述符格式
// - TTBR0/TTBR1管理
// - MAIR_EL1配置
// - TCR_EL1配置
// - 4KB/16KB/64KB页支持
// - 2MB/1GB大页支持
// - ASID管理
// - TLB失效操作

// 需要实现的具体类型（示例）：
// pub struct Aarch64Module;
// pub struct Aarch64ModuleInfo;
// pub struct Aarch64ModuleConfig;
//
// 需要实现的方法（示例）：
// pub fn init() -> ArchResult<()>;
// pub fn create(config: &Aarch64ModuleConfig) -> ArchResult<Aarch64Module>;
// pub fn info(&self) -> Aarch64ModuleInfo;
//
// 寄存器定义（示例）：
// use tock_registers::{register_bitfields, register_structs, registers::*};
//
// 对象管理（示例）：
// use crate::object::{KernelObject, Handle, ObjectRights};
//
// 遵循"一切皆对象"原则，所有资源都封装为对象。

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::arch::traits::mmu::*;
use crate::arch::{ArchError, ArchResult};
use shared::sync::mutex::Mutex;

const PHYS_BASE: usize = 0x4000_0000;
const DEFAULT_PHYS_MEM_SIZE: usize = 512 * 1024 * 1024;

static NEXT_FREE_PADDR: AtomicUsize = AtomicUsize::new(0);

const MAX_MAPPINGS: usize = 256;
static MAPPINGS: Mutex<[Option<MappingEntry>; MAX_MAPPINGS]> = Mutex::new([None; MAX_MAPPINGS]);

#[derive(Debug, Clone, Copy)]
struct MappingEntry {
    vaddr: usize,
    paddr: usize,
    size: usize,
    flags: MappingFlags,
}

#[derive(Debug, Clone, Copy)]
pub struct Aarch64MemoryRegion {
    base: usize,
    size: usize,
    memory_type: MemoryType,
    cacheable: bool,
    executable: bool,
    writable: bool,
    device: bool,
}

impl Aarch64MemoryRegion {
    pub const fn new(base: usize, size: usize, memory_type: MemoryType) -> Self {
        let device = matches!(memory_type, MemoryType::Device);
        Self {
            base,
            size,
            memory_type,
            cacheable: !device,
            executable: !device,
            writable: true,
            device,
        }
    }
}

impl MemoryRegion for Aarch64MemoryRegion {
    fn base(&self) -> usize {
        self.base
    }

    fn size(&self) -> usize {
        self.size
    }

    fn memory_type(&self) -> MemoryType {
        self.memory_type
    }

    fn is_cacheable(&self) -> bool {
        self.cacheable
    }

    fn is_executable(&self) -> bool {
        self.executable
    }

    fn is_writable(&self) -> bool {
        self.writable
    }

    fn is_device(&self) -> bool {
        self.device
    }
}

#[derive(Debug, Clone)]
pub struct Aarch64PageTable {
    root: usize,
    asid: u16,
}

impl Aarch64PageTable {
    pub const fn empty() -> Self {
        Self { root: 0, asid: 0 }
    }
}

impl PageTable for Aarch64PageTable {
    fn root_address(&self) -> usize {
        self.root
    }

    fn asid(&self) -> u16 {
        self.asid
    }

    fn set_asid(&mut self, asid: u16) {
        self.asid = asid;
    }

    fn clone_with_asid(&self, asid: u16) -> ArchResult<Self> {
        Ok(Self {
            root: self.root,
            asid,
        })
    }

    fn stats(&self) -> PageTableStats {
        PageTableStats {
            total_pages: 0,
            used_pages: 0,
            huge_pages: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Aarch64PageTableEntry {
    paddr: usize,
    flags: EntryFlags,
    valid: bool,
}

impl PageTableEntry for Aarch64PageTableEntry {
    fn invalid() -> Self {
        Self {
            paddr: 0,
            flags: EntryFlags {
                present: false,
                writable: false,
                executable: false,
                user: false,
                global: false,
                accessed: false,
                dirty: false,
                huge_page: false,
            },
            valid: false,
        }
    }

    fn new(paddr: usize, flags: EntryFlags) -> Self {
        Self {
            paddr,
            flags,
            valid: flags.present,
        }
    }

    fn is_valid(&self) -> bool {
        self.valid
    }

    fn physical_address(&self) -> usize {
        self.paddr
    }

    fn flags(&self) -> EntryFlags {
        self.flags
    }

    fn set_accessed(&mut self, accessed: bool) {
        self.flags.accessed = accessed;
    }

    fn set_dirty(&mut self, dirty: bool) {
        self.flags.dirty = dirty;
    }
}

#[derive(Debug)]
pub struct Aarch64AddressSpace {
    table: Aarch64PageTable,
}

impl AddressSpace for Aarch64AddressSpace {
    fn new() -> ArchResult<Self> {
        Ok(Self {
            table: create_page_table()?,
        })
    }

    fn map_region(
        &mut self,
        vaddr: usize,
        region: &dyn MemoryRegion,
        flags: MappingFlags,
    ) -> ArchResult<()> {
        map(&mut self.table, vaddr, region.base(), region.size(), flags)
    }

    fn allocate(
        &mut self,
        _size: usize,
        _alignment: usize,
        _flags: AllocationFlags,
    ) -> ArchResult<VirtualMemory> {
        Err(ArchError::NoMemory)
    }

    fn free(&mut self, _vmem: VirtualMemory) -> ArchResult<()> {
        Err(ArchError::NotSupported)
    }
}

#[derive(Debug)]
pub struct Aarch64Mmu;

pub fn init() -> ArchResult<()> {
    ensure_phys_alloc_initialized();
    Ok(())
}

pub fn create_page_table() -> ArchResult<Aarch64PageTable> {
    let root = allocate_physical_page()?;
    Ok(Aarch64PageTable { root, asid: 0 })
}

pub fn current_page_table() -> Aarch64PageTable {
    Aarch64PageTable::empty()
}

pub fn switch_page_table(_table: &Aarch64PageTable) {}

pub fn map(
    _table: &mut Aarch64PageTable,
    vaddr: usize,
    paddr: usize,
    size: usize,
    flags: MappingFlags,
) -> ArchResult<()> {
    if size == 0 {
        return Err(ArchError::InvalidArgument);
    }

    let mut table = MAPPINGS.lock();
    for slot in table.iter_mut() {
        if slot.is_none() {
            *slot = Some(MappingEntry {
                vaddr,
                paddr,
                size,
                flags,
            });
            return Ok(());
        }
    }
    Err(ArchError::NoMemory)
}

pub fn unmap(_table: &mut Aarch64PageTable, vaddr: usize, size: usize) -> ArchResult<()> {
    let mut table = MAPPINGS.lock();
    for slot in table.iter_mut() {
        if let Some(existing) = slot {
            if existing.vaddr == vaddr && existing.size == size {
                *slot = None;
                return Ok(());
            }
        }
    }
    Err(ArchError::NotMapped)
}

pub fn query(_table: &Aarch64PageTable, vaddr: usize) -> ArchResult<MappingInfo> {
    let table = MAPPINGS.lock();
    for slot in table.iter() {
        if let Some(existing) = slot {
            if vaddr >= existing.vaddr && vaddr < existing.vaddr.saturating_add(existing.size) {
                let offset = vaddr - existing.vaddr;
                return Ok(MappingInfo {
                    paddr: existing.paddr.saturating_add(offset),
                    size: existing.size.saturating_sub(offset),
                    flags: existing.flags,
                });
            }
        }
    }
    Err(ArchError::NotMapped)
}

pub fn tlb_flush(_vaddr: Option<usize>, _asid: Option<u16>) {}

pub fn cache_invalidate(_vaddr: usize, _size: usize) {}

pub fn page_size() -> usize {
    4096
}

pub fn supported_huge_page_sizes() -> &'static [usize] {
    const SIZES: [usize; 2] = [2 * 1024 * 1024, 1024 * 1024 * 1024];
    &SIZES
}

pub fn allocate_physical_page() -> ArchResult<usize> {
    ensure_phys_alloc_initialized();
    let paddr = NEXT_FREE_PADDR.fetch_add(page_size(), Ordering::Relaxed);
    if paddr == 0 {
        return Err(ArchError::InternalError);
    }
    let limit = PHYS_BASE.saturating_add(DEFAULT_PHYS_MEM_SIZE);
    if paddr.saturating_add(page_size()) > limit {
        return Err(ArchError::NoMemory);
    }
    Ok(paddr)
}

pub fn free_physical_page(_paddr: usize) -> ArchResult<()> {
    Ok(())
}

fn ensure_phys_alloc_initialized() {
    if NEXT_FREE_PADDR.load(Ordering::Relaxed) != 0 {
        return;
    }

    extern "C" {
        static __kernel_end: u8;
    }

    let start = crate::arch::common::align_to_page(unsafe { &__kernel_end as *const u8 as usize });
    let start = start.max(PHYS_BASE);
    let _ = NEXT_FREE_PADDR.compare_exchange(0, start, Ordering::Relaxed, Ordering::Relaxed);
}
