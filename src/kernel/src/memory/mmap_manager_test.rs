//! 内存映射管理器测试

#![cfg(test)]

use super::mmap_manager::*;

#[test]
fn test_memory_map_entry_creation() {
    let entry = MemoryMapEntry::new(
        0x1000,
        0x1000,
        MemoryMapType::Ram,
        MemoryMapFlags::ram_default(),
    );
    
    assert_eq!(entry.range.start, 0x1000);
    assert_eq!(entry.range.end, 0x2000);
    assert_eq!(entry.size(), 0x1000);
    assert_eq!(entry.map_type, MemoryMapType::Ram);
    assert_eq!(entry.flags, MemoryMapFlags::ram_default());
}

#[test]
fn test_memory_map_entry_overlap() {
    let entry1 = MemoryMapEntry::new(
        0x1000,
        0x1000,
        MemoryMapType::Ram,
        MemoryMapFlags::ram_default(),
    );
    
    let entry2 = MemoryMapEntry::new(
        0x1800,
        0x1000,
        MemoryMapType::Rom,
        MemoryMapFlags::rom_default(),
    );
    
    let entry3 = MemoryMapEntry::new(
        0x3000,
        0x1000,
        MemoryMapType::Device,
        MemoryMapFlags::device_default(),
    );
    
    assert!(entry1.overlaps(&entry2));
    assert!(!entry1.overlaps(&entry3));
    assert!(!entry2.overlaps(&entry3));
}

#[test]
fn test_memory_map_manager_add_entry() {
    let mut manager = MemoryMapManager::new();
    
    let entry1 = MemoryMapEntry::new(
        0x1000,
        0x1000,
        MemoryMapType::Ram,
        MemoryMapFlags::ram_default(),
    );
    
    let entry2 = MemoryMapEntry::new(
        0x3000,
        0x1000,
        MemoryMapType::Rom,
        MemoryMapFlags::rom_default(),
    );
    
    assert!(manager.add_entry(entry1.clone()).is_ok());
    assert!(manager.add_entry(entry2.clone()).is_ok());
    assert_eq!(manager.entries().len(), 2);
}

#[test]
fn test_memory_map_manager_add_conflicting_entry() {
    let mut manager = MemoryMapManager::new();
    
    let entry1 = MemoryMapEntry::new(
        0x1000,
        0x1000,
        MemoryMapType::Ram,
        MemoryMapFlags::ram_default(),
    );
    
    let entry2 = MemoryMapEntry::new(
        0x1800,
        0x1000,
        MemoryMapType::Rom,
        MemoryMapFlags::rom_default(),
    );
    
    assert!(manager.add_entry(entry1.clone()).is_ok());
    assert!(manager.add_entry(entry2.clone()).is_err());
    assert_eq!(manager.entries().len(), 1);
}

#[test]
fn test_memory_map_manager_find_entry() {
    let mut manager = MemoryMapManager::new();
    
    let entry = MemoryMapEntry::new(
        0x1000,
        0x1000,
        MemoryMapType::Ram,
        MemoryMapFlags::ram_default(),
    );
    
    assert!(manager.add_entry(entry.clone()).is_ok());
    
    assert!(manager.find_entry(0x1500).is_some());
    assert!(manager.find_entry(0x500).is_none());
    assert!(manager.find_entry(0x2500).is_none());
}

#[test]
fn test_memory_map_manager_remove_entries() {
    let mut manager = MemoryMapManager::new();
    
    let entry1 = MemoryMapEntry::new(
        0x1000,
        0x1000,
        MemoryMapType::Ram,
        MemoryMapFlags::ram_default(),
    );
    
    let entry2 = MemoryMapEntry::new(
        0x3000,
        0x1000,
        MemoryMapType::Rom,
        MemoryMapFlags::rom_default(),
    );
    
    assert!(manager.add_entry(entry1.clone()).is_ok());
    assert!(manager.add_entry(entry2.clone()).is_ok());
    
    let removed = manager.remove_entries_in_range(0x500..0x2000);
    assert_eq!(removed, 1);
    assert_eq!(manager.entries().len(), 1);
}

#[test]
fn test_memory_map_flags_presets() {
    let ram_flags = MemoryMapFlags::ram_default();
    assert!(ram_flags.readable);
    assert!(ram_flags.writable);
    assert!(!ram_flags.executable);
    assert!(!ram_flags.user_accessible);
    assert!(ram_flags.cacheable);
    
    let rom_flags = MemoryMapFlags::rom_default();
    assert!(rom_flags.readable);
    assert!(!rom_flags.writable);
    assert!(!rom_flags.executable);
    assert!(rom_flags.user_accessible);
    assert!(rom_flags.cacheable);
    
    let device_flags = MemoryMapFlags::device_default();
    assert!(device_flags.readable);
    assert!(device_flags.writable);
    assert!(!device_flags.executable);
    assert!(!device_flags.user_accessible);
    assert!(!device_flags.cacheable);
}

#[test]
fn test_memory_map_manager_update_flags_in_range() {
    let mut manager = MemoryMapManager::new();
    
    let entry = MemoryMapEntry::new(
        0x1000,
        0x2000,
        MemoryMapType::Ram,
        MemoryMapFlags::ram_default(),
    );
    
    assert!(manager.add_entry(entry.clone()).is_ok());
    
    let new_flags = MemoryMapFlags::new(true, false, true, true, true); // readable, executable, user_accessible, cacheable
    let updated = manager.update_flags_in_range(0x1000..0x3000, new_flags);
    assert!(updated.is_ok());
    assert_eq!(updated.unwrap(), 1);
    
    // Verify the flags were updated
    let found_entry = manager.find_entry(0x1500).unwrap();
    assert_eq!(found_entry.flags, new_flags);
}

#[test]
fn test_memory_map_manager_find_overlapping_entries() {
    let mut manager = MemoryMapManager::new();
    
    let entry1 = MemoryMapEntry::new(
        0x1000,
        0x1000,
        MemoryMapType::Ram,
        MemoryMapFlags::ram_default(),
    );
    
    let entry2 = MemoryMapEntry::new(
        0x3000,
        0x1000,
        MemoryMapType::Rom,
        MemoryMapFlags::rom_default(),
    );
    
    assert!(manager.add_entry(entry1.clone()).is_ok());
    assert!(manager.add_entry(entry2.clone()).is_ok());
    
    // Find entries overlapping with 0x1800..0x2800 (should overlap with entry1)
    let overlapping = manager.find_overlapping_entries(0x1800..0x2800);
    assert_eq!(overlapping.len(), 1);
    assert_eq!(overlapping[0].range, entry1.range);
    
    // Find entries overlapping with 0x500..0x4000 (should overlap with both entries)
    let overlapping = manager.find_overlapping_entries(0x500..0x4000);
    assert_eq!(overlapping.len(), 2);
    
    // Find entries overlapping with 0x5000..0x6000 (should overlap with neither)
    let overlapping = manager.find_overlapping_entries(0x5000..0x6000);
    assert_eq!(overlapping.len(), 0);
}

#[test]
fn test_memory_map_manager_update_flags_with_partial_overlap() {
    let mut manager = MemoryMapManager::new();
    
    let entry = MemoryMapEntry::new(
        0x1000,
        0x3000, // 0x1000 to 0x4000
        MemoryMapType::Ram,
        MemoryMapFlags::ram_default(),
    );
    
    assert!(manager.add_entry(entry.clone()).is_ok());
    
    // Update flags for the middle part (0x2000..0x3000)
    let new_flags = MemoryMapFlags::new(true, false, true, true, true); // readable, executable, user_accessible, cacheable
    let result = manager.update_flags_with_partial_overlap(0x2000..0x3000, new_flags);
    assert!(result.is_ok());
    
    // Should now have 3 entries:
    // 1. 0x1000..0x2000 with original flags
    // 2. 0x2000..0x3000 with new flags
    // 3. 0x3000..0x4000 with original flags
    
    assert_eq!(manager.entries().len(), 3);
    
    // Verify the entries
    let mut entries: Vec<&MemoryMapEntry> = manager.entries().iter().collect();
    entries.sort_by_key(|e| e.range.start);
    
    // First entry (original flags)
    assert_eq!(entries[0].range.start, 0x1000);
    assert_eq!(entries[0].range.end, 0x2000);
    assert_eq!(entries[0].flags, MemoryMapFlags::ram_default());
    
    // Middle entry (new flags)
    assert_eq!(entries[1].range.start, 0x2000);
    assert_eq!(entries[1].range.end, 0x3000);
    assert_eq!(entries[1].flags, new_flags);
    
    // Last entry (original flags)
    assert_eq!(entries[2].range.start, 0x3000);
    assert_eq!(entries[2].range.end, 0x4000);
    assert_eq!(entries[2].flags, MemoryMapFlags::ram_default());
}