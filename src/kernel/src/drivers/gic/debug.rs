//! GIC debugging interface
//!
//! This module provides debugging utilities for inspecting GIC state,
//! interrupt priorities, and interrupt statistics.

use super::stats::IRQ_STATS;

/// Dump complete GIC state to log
pub fn dump_gic_state() {
    crate::info!("=== GIC State ===");
    crate::info!("Running priority: {}", super::get_running_priority());
    crate::info!("Highest pending priority: {}", super::get_highest_pending_priority());

    crate::info!("\nConfigured Priorities:");
    // Show priorities for known system interrupts
    let system_irqs = [30, 33];  // Timer, UART
    for &irq in &system_irqs {
        let priority = super::get_interrupt_priority(irq);
        crate::info!("  IRQ {}: priority {} ({})", 
            irq, priority, priority_name(priority));
    }
}

/// Dump interrupt statistics
pub fn dump_interrupt_stats() {
    super::stats::dump_interrupt_stats();
}

/// Dump comprehensive interrupt information
pub fn dump_all() {
    crate::info!("========================================");
    crate::info!("  GIC Interrupt Priority Debug Report");
    crate::info!("========================================\n");
    
    dump_gic_state();
    crate::info!("");
    dump_interrupt_stats();
    
    crate::info!("\n========================================");
}

/// Dump priority information for a specific interrupt
pub fn dump_irq_info(irq: u32) {
    let priority = super::get_interrupt_priority(irq);
    let count = IRQ_STATS.lock().get_count(irq);
    
    crate::info!("=== IRQ {} Information ===", irq);
    crate::info!("Priority: {} ({})", priority, priority_name(priority));
    crate::info!("Total count: {}", count);
}

/// Dump all active interrupts and their priorities
pub fn dump_active_irqs() {
    crate::info!("=== Active Interrupts ===");
    
    let stats = IRQ_STATS.lock();
    let active_irqs = stats.get_active_irqs();
    
    if active_irqs.is_empty() {
        crate::info!("No interrupts have occurred yet");
        return;
    }
    
    crate::info!("IRQ | Priority | Name         | Count");
    crate::info!("----+----------+--------------+-------");
    
    for (irq, count) in active_irqs {
        let priority = super::get_interrupt_priority(irq);
        crate::info!("{:3} | {:8} | {:12} | {}", 
            irq, priority, priority_name(priority), count);
    }
}

/// Get human-readable name for priority level
fn priority_name(priority: u8) -> &'static str {
    match priority {
        super::PRIORITY_CRITICAL => "CRITICAL",
        super::PRIORITY_VERY_HIGH => "VERY_HIGH",
        super::PRIORITY_HIGH => "HIGH",
        super::PRIORITY_NORMAL => "NORMAL",
        super::PRIORITY_LOW => "LOW",
        super::PRIORITY_LOWEST => "LOWEST",
        _ => "CUSTOM",
    }
}

/// Verify priority configuration is correct
pub fn verify_priorities() -> bool {
    let mut all_ok = true;
    
    crate::info!("=== Verifying Interrupt Priorities ===");
    
    // Check timer interrupt (should be VERY_HIGH)
    let timer_priority = super::get_interrupt_priority(30);
    if timer_priority != super::PRIORITY_VERY_HIGH {
        crate::warn!("Timer interrupt (30) has incorrect priority: {} (expected {})",
            timer_priority, super::PRIORITY_VERY_HIGH);
        all_ok = false;
    } else {
        crate::info!("✓ Timer interrupt (30): priority {} (VERY_HIGH)", timer_priority);
    }
    
    // Check UART interrupt (should be HIGH)
    let uart_priority = super::get_interrupt_priority(33);
    if uart_priority != super::PRIORITY_HIGH {
        crate::warn!("UART interrupt (33) has incorrect priority: {} (expected {})",
            uart_priority, super::PRIORITY_HIGH);
        all_ok = false;
    } else {
        crate::info!("✓ UART interrupt (33): priority {} (HIGH)", uart_priority);
    }
    
    if all_ok {
        crate::info!("All priority configurations verified successfully");
    } else {
        crate::warn!("Some priority configurations are incorrect");
    }
    
    all_ok
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_priority_name() {
        assert_eq!(priority_name(super::super::PRIORITY_CRITICAL), "CRITICAL");
        assert_eq!(priority_name(super::super::PRIORITY_VERY_HIGH), "VERY_HIGH");
        assert_eq!(priority_name(super::super::PRIORITY_HIGH), "HIGH");
        assert_eq!(priority_name(super::super::PRIORITY_NORMAL), "NORMAL");
        assert_eq!(priority_name(super::super::PRIORITY_LOW), "LOW");
        assert_eq!(priority_name(super::super::PRIORITY_LOWEST), "LOWEST");
        assert_eq!(priority_name(0x50), "CUSTOM");
    }
}
