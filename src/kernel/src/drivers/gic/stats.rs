//! Interrupt statistics tracking module
//!
//! This module provides detailed statistics tracking for interrupt handling,
//! including per-IRQ counts, preemption tracking, and nesting level monitoring.

use spin::Mutex;
extern crate alloc;
use alloc::vec::Vec;

/// Maximum number of interrupt IDs to track (GICv2 supports up to 1020)
const MAX_IRQS: usize = 1020;

/// Interrupt statistics collector
pub struct InterruptStats {
    /// Per-IRQ interrupt counts
    pub counts: [u64; MAX_IRQS],
    /// Number of times a higher-priority interrupt preempted a lower-priority one
    pub preemptions: u64,
    /// Maximum interrupt nesting level observed
    pub max_nesting: usize,
    /// Current interrupt nesting level
    pub current_nesting: usize,
    /// Total number of interrupts processed
    pub total_irqs: u64,
}

impl InterruptStats {
    /// Create a new interrupt statistics tracker
    const fn new() -> Self {
        Self {
            counts: [0; MAX_IRQS],
            preemptions: 0,
            max_nesting: 0,
            current_nesting: 0,
            total_irqs: 0,
        }
    }

    /// Record an interrupt occurrence
    ///
    /// # Arguments
    /// * `irq` - The interrupt ID that occurred
    /// * `was_preemption` - Whether this interrupt preempted another handler
    pub fn record_irq(&mut self, irq: u32, was_preemption: bool) {
        // Update per-IRQ count
        if (irq as usize) < MAX_IRQS {
            self.counts[irq as usize] += 1;
        }

        // Track preemptions
        if was_preemption {
            self.preemptions += 1;
        }

        // Update nesting level
        self.current_nesting += 1;
        if self.current_nesting > self.max_nesting {
            self.max_nesting = self.current_nesting;
        }

        // Total count
        self.total_irqs += 1;
    }

    /// Record exit from interrupt handler
    pub fn exit_irq(&mut self) {
        if self.current_nesting > 0 {
            self.current_nesting -= 1;
        }
    }

    /// Get interrupt count for a specific IRQ
    pub fn get_count(&self, irq: u32) -> u64 {
        if (irq as usize) < MAX_IRQS {
            self.counts[irq as usize]
        } else {
            0
        }
    }

    /// Get all active IRQs (IRQs with non-zero counts)
    pub fn get_active_irqs(&self) -> Vec<(u32, u64)> {
        let mut active = Vec::new();
        for irq in 0..MAX_IRQS {
            if self.counts[irq] > 0 {
                active.push((irq as u32, self.counts[irq]));
            }
        }
        active
    }

    /// Reset all statistics
    pub fn reset(&mut self) {
        self.counts = [0; MAX_IRQS];
        self.preemptions = 0;
        self.max_nesting = 0;
        self.current_nesting = 0;
        self.total_irqs = 0;
    }
}

/// Global interrupt statistics
pub static IRQ_STATS: Mutex<InterruptStats> = Mutex::new(InterruptStats::new());

/// Dump interrupt statistics to log
pub fn dump_interrupt_stats() {
    let stats = IRQ_STATS.lock();

    crate::info!("=== Interrupt Statistics ===");
    crate::info!("Total IRQs: {}", stats.total_irqs);
    crate::info!("Preemptions: {}", stats.preemptions);
    crate::info!("Max nesting: {}", stats.max_nesting);
    crate::info!("Current nesting: {}", stats.current_nesting);

    crate::info!("\nPer-IRQ Counts:");
    for irq in 0..MAX_IRQS {
        if stats.counts[irq] > 0 {
            crate::info!("  IRQ {}: {} interrupts", irq, stats.counts[irq]);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_record_irq() {
        let mut stats = InterruptStats::new();
        
        // Record normal interrupt
        stats.record_irq(30, false);
        assert_eq!(stats.get_count(30), 1);
        assert_eq!(stats.total_irqs, 1);
        assert_eq!(stats.preemptions, 0);
        assert_eq!(stats.current_nesting, 1);
        
        // Record preemption
        stats.record_irq(31, true);
        assert_eq!(stats.get_count(31), 1);
        assert_eq!(stats.total_irqs, 2);
        assert_eq!(stats.preemptions, 1);
        assert_eq!(stats.current_nesting, 2);
        assert_eq!(stats.max_nesting, 2);
    }

    #[test]
    fn test_exit_irq() {
        let mut stats = InterruptStats::new();
        
        stats.record_irq(30, false);
        assert_eq!(stats.current_nesting, 1);
        
        stats.exit_irq();
        assert_eq!(stats.current_nesting, 0);
        
        // Should not go negative
        stats.exit_irq();
        assert_eq!(stats.current_nesting, 0);
    }

    #[test]
    fn test_reset() {
        let mut stats = InterruptStats::new();
        
        stats.record_irq(30, false);
        stats.record_irq(31, true);
        
        stats.reset();
        assert_eq!(stats.get_count(30), 0);
        assert_eq!(stats.get_count(31), 0);
        assert_eq!(stats.total_irqs, 0);
        assert_eq!(stats.preemptions, 0);
    }
}