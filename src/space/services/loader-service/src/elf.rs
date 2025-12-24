use core::slice;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Header {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64ProgramHeader {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

pub const PT_LOAD: u32 = 1;
pub const PF_X: u32 = 0x1;
pub const PF_W: u32 = 0x2;
pub const PF_R: u32 = 0x4;

pub const STACK_SIZE: usize = 8192;

pub struct ElfLoader<'a> {
    data: &'a [u8],
}

impl<'a> ElfLoader<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self, &'static str> {
        if data.len() < core::mem::size_of::<Elf64Header>() {
            return Err("ELF file too small");
        }
        
        if &data[0..4] != b"\x7fELF" {
            return Err("Invalid ELF magic");
        }
        
        Ok(Self { data })
    }
    
    pub fn header(&self) -> &Elf64Header {
        unsafe { &*(self.data.as_ptr() as *const Elf64Header) }
    }
    
    pub fn program_headers(&self) -> &[Elf64ProgramHeader] {
        let header = self.header();
        let phoff = header.e_phoff as usize;
        let phnum = header.e_phnum as usize;
        
        if phoff + phnum * core::mem::size_of::<Elf64ProgramHeader>() > self.data.len() {
            return &[];
        }
        
        unsafe {
            slice::from_raw_parts(
                self.data.as_ptr().add(phoff) as *const Elf64ProgramHeader,
                phnum
            )
        }
    }
    
    pub fn entry(&self) -> u64 {
        self.header().e_entry
    }
    
    pub fn segment_data(&self, ph: &Elf64ProgramHeader) -> &[u8] {
        let offset = ph.p_offset as usize;
        let size = ph.p_filesz as usize;
        
        if offset + size > self.data.len() {
            return &[];
        }
        
        &self.data[offset..offset + size]
    }
}

pub struct LoadedProgram {
    pub entry: usize,
    pub stack_top: usize,
    pub segments: [(usize, usize, u32); 8],
    pub segment_count: usize,
}

impl LoadedProgram {
    pub fn new() -> Self {
        Self {
            entry: 0,
            stack_top: 0,
            segments: [(0, 0, 0); 8],
            segment_count: 0,
        }
    }
}

pub fn parse_elf(elf_data: &[u8]) -> Result<LoadedProgram, &'static str> {
    let loader = ElfLoader::new(elf_data)?;
    
    let mut program = LoadedProgram::new();
    program.entry = loader.entry() as usize;
    
    for ph in loader.program_headers() {
        if ph.p_type == PT_LOAD {
            if program.segment_count >= 8 {
                return Err("Too many segments");
            }
            
            let vaddr = ph.p_vaddr as usize;
            let memsz = ph.p_memsz as usize;
            let flags = ph.p_flags;
            
            program.segments[program.segment_count] = (vaddr, memsz, flags);
            program.segment_count += 1;
        }
    }
    
    program.stack_top = 0x220000 + STACK_SIZE;
    
    Ok(program)
}
