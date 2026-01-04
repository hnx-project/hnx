//! Managers module for user-space filesystem server

pub mod ramfs_manager;
pub mod cpio_manager;
pub mod elf_manager;

/// Initialize all managers
pub fn init() {
    crate::println!("Initializing filesystem managers...");
    // Manager initialization would happen here
}