//! Tests for interrupt priority management

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drivers::gic::*;

    #[test]
    fn test_priority_constants() {
        // Test that priority constants are defined correctly
        assert_eq!(PRIORITY_CRITICAL, 0x00);
        assert_eq!(PRIORITY_VERY_HIGH, 0x20);
        assert_eq!(PRIORITY_HIGH, 0x40);
        assert_eq!(PRIORITY_NORMAL, 0x80);
        assert_eq!(PRIORITY_LOW, 0xC0);
        assert_eq!(PRIORITY_LOWEST, 0xE0);
        
        // Verify priority ordering (lower value = higher priority)
        assert!(PRIORITY_CRITICAL < PRIORITY_VERY_HIGH);
        assert!(PRIORITY_VERY_HIGH < PRIORITY_HIGH);
        assert!(PRIORITY_HIGH < PRIORITY_NORMAL);
        assert!(PRIORITY_NORMAL < PRIORITY_LOW);
        assert!(PRIORITY_LOW < PRIORITY_LOWEST);
    }

    #[test]
    fn test_binary_point_constants() {
        assert_eq!(BINARY_POINT_PREEMPT_ALL, 0b000);
        assert_eq!(BINARY_POINT_PREEMPT_NONE, 0b111);
    }

    #[test]
    fn test_priority_mask_constants() {
        assert_eq!(PRIORITY_MASK_NONE, 0xFF);
        assert_eq!(PRIORITY_MASK_HIGH, 0x40);
    }
}