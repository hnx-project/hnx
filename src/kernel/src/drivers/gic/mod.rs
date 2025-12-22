pub mod v2;
pub mod stats;
pub mod debug;

use crate::arch::common::traits::InterruptController;

pub fn init() { v2::GicV2Controller::init() }
pub fn enable_irq(irq: u32) { v2::GicV2Controller::enable_irq(irq) }
pub fn disable_irq(irq: u32) { v2::GicV2Controller::disable_irq(irq) }
pub fn ack_irq(irq: u32) { v2::GicV2Controller::ack_irq(irq) }
pub fn read_iar() -> u32 { v2::read_iar() }
pub fn set_bases(gicd: usize, gicc: usize) { v2::set_bases(gicd, gicc) }
pub fn write_eoi(iar: u32) { v2::write_eoi(iar) }

// Priority management functions
pub fn set_interrupt_priority(irq: u32, priority: u8) { 
    v2::set_interrupt_priority(irq, priority) 
}
pub fn get_interrupt_priority(irq: u32) -> u8 { 
    v2::get_interrupt_priority(irq) 
}
pub fn set_binary_point(bp: u8) { 
    v2::set_binary_point(bp) 
}
pub fn set_priority_mask(mask: u8) { 
    v2::set_priority_mask(mask) 
}
pub fn get_running_priority() -> u8 { 
    v2::get_running_priority() 
}
pub fn get_highest_pending_priority() -> u8 { 
    v2::get_highest_pending_priority() 
}

// Re-export priority constants
pub use v2::{
    PRIORITY_CRITICAL, PRIORITY_VERY_HIGH, PRIORITY_HIGH, 
    PRIORITY_NORMAL, PRIORITY_LOW, PRIORITY_LOWEST,
    BINARY_POINT_PREEMPT_ALL, BINARY_POINT_PREEMPT_NONE,
    PRIORITY_MASK_NONE, PRIORITY_MASK_HIGH,
};