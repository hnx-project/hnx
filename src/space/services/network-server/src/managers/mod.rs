//! Network managers for user-space network server

pub mod socket_manager;
pub mod interface_manager;

/// Initialize all managers
pub fn init() {
    crate::println!("Initializing network managers...");
    // Manager initialization would happen here
}