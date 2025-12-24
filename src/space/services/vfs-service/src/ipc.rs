use crate::ramfs::RamFs;
use hnx_abi::*;

const VFS_OP_OPEN: u16 = 100;
const VFS_OP_READ: u16 = 101;
const VFS_OP_WRITE: u16 = 102;
const VFS_OP_CLOSE: u16 = 103;
const VFS_OP_CREAT: u16 = 104;
const VFS_OP_UNLINK: u16 = 105;
const VFS_OP_MKDIR: u16 = 106;
const VFS_OP_RMDIR: u16 = 107;

pub struct VfsIpcHandler {
    fs: RamFs,
}

impl VfsIpcHandler {
    pub fn new(fs: RamFs) -> Self {
        Self { fs }
    }
    
    pub fn wait_and_handle(&mut self, _epid: u32) -> Result<(), ()> {
        Ok(())
    }
    
    fn handle_open(&mut self, _msg: &[u8]) -> Result<i32, ()> {
        Ok(0)
    }
    
    fn handle_read(&mut self, msg: &[u8]) -> Result<usize, ()> {
        if msg.len() < 8 {
            return Err(());
        }
        
        Ok(0)
    }
    
    fn handle_write(&mut self, msg: &[u8]) -> Result<usize, ()> {
        if msg.len() < 8 {
            return Err(());
        }
        
        Ok(0)
    }
    
    fn handle_close(&mut self, _msg: &[u8]) -> Result<(), ()> {
        Ok(())
    }
    
    fn handle_creat(&mut self, msg: &[u8]) -> Result<i32, ()> {
        if msg.is_empty() {
            return Err(());
        }
        
        let path_str = core::str::from_utf8(msg).map_err(|_| ())?;
        let _inode = self.fs.create(path_str)?;
        Ok(0)
    }
    
    fn handle_unlink(&mut self, msg: &[u8]) -> Result<(), ()> {
        if msg.is_empty() {
            return Err(());
        }
        
        let path_str = core::str::from_utf8(msg).map_err(|_| ())?;
        self.fs.unlink(path_str)
    }
    
    fn handle_mkdir(&mut self, msg: &[u8]) -> Result<(), ()> {
        if msg.is_empty() {
            return Err(());
        }
        
        let path_str = core::str::from_utf8(msg).map_err(|_| ())?;
        let _inode = self.fs.mkdir(path_str)?;
        Ok(())
    }
    
    fn handle_rmdir(&mut self, msg: &[u8]) -> Result<(), ()> {
        if msg.is_empty() {
            return Err(());
        }
        
        let path_str = core::str::from_utf8(msg).map_err(|_| ())?;
        self.fs.unlink(path_str)
    }
}
