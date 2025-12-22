//! Tests for the block device driver interface

#![cfg(test)]

extern crate alloc;

use crate::drivers::block::*;
use crate::drivers::ipc_protocol::{DeviceId, DriverId};

#[test]
fn test_sd_mmc_driver_initialization() {
    let device_info = BlockDeviceInfo {
        device_id: DeviceId(1),
        driver_id: DriverId(1),
        block_size: 512,
        total_blocks: 1024 * 1024, // 512MB card
        device_name: "Test SD Card".into(),
        device_type: BlockDeviceType::SdCard,
    };
    
    let driver = SdMmcDriver::init(device_info.clone());
    assert!(driver.is_ok());
    
    let driver = driver.unwrap();
    assert_eq!(driver.device_info.device_id, DeviceId(1));
    assert_eq!(driver.device_info.block_size, 512);
    assert_eq!(driver.device_info.total_blocks, 1024 * 1024);
    assert_eq!(driver.device_info.device_type, BlockDeviceType::SdCard);
}

#[test]
fn test_block_read_write_operations() {
    let device_info = BlockDeviceInfo {
        device_id: DeviceId(1),
        driver_id: DriverId(1),
        block_size: 512,
        total_blocks: 1024 * 1024,
        device_name: "Test SD Card".into(),
        device_type: BlockDeviceType::SdCard,
    };
    
    let mut driver = SdMmcDriver::init(device_info).unwrap();
    
    // Test read operation
    let mut read_buffer = vec![0u8; 1024]; // 2 blocks
    let read_result = driver.read_blocks(0, 2, &mut read_buffer);
    assert!(read_result.is_ok());
    assert_eq!(read_result.unwrap(), 2);
    
    // Verify buffer was filled with data
    // Note: In our implementation, we fill with sequential values
    assert_eq!(read_buffer[0], 0);
    assert_eq!(read_buffer[1], 1);
    assert_eq!(read_buffer[256], 0); // Next block starts again from 0
    
    // Test write operation
    let write_data = vec![0xFFu8; 1024]; // 2 blocks of 0xFF
    let write_result = driver.write_blocks(0, 2, &write_data);
    assert!(write_result.is_ok());
    assert_eq!(write_result.unwrap(), 2);
}

#[test]
fn test_block_ioctl_operations() {
    let device_info = BlockDeviceInfo {
        device_id: DeviceId(1),
        driver_id: DriverId(1),
        block_size: 512,
        total_blocks: 1024 * 1024,
        device_name: "Test SD Card".into(),
        device_type: BlockDeviceType::SdCard,
    };
    
    let mut driver = SdMmcDriver::init(device_info).unwrap();
    
    // Test getting OCR register
    let ocr_result = driver.ioctl(0x1001, 0);
    assert!(ocr_result.is_ok());
    assert_eq!(ocr_result.unwrap(), driver.ocr as u64);
    
    // Test getting capacity
    let capacity_result = driver.ioctl(0x1002, 0);
    assert!(capacity_result.is_ok());
    assert_eq!(capacity_result.unwrap(), driver.device_info.total_blocks);
    
    // Test invalid command
    let invalid_result = driver.ioctl(0xFFFF, 0);
    assert!(invalid_result.is_err());
}

#[test]
fn test_sd_mmc_card_detection() {
    let device_info = BlockDeviceInfo {
        device_id: DeviceId(1),
        driver_id: DriverId(1),
        block_size: 512,
        total_blocks: 1024 * 1024,
        device_name: "Test SD Card".into(),
        device_type: BlockDeviceType::SdCard,
    };
    
    let driver = SdMmcDriver::init(device_info).unwrap();
    
    // Test card presence (always true in our implementation)
    assert!(driver.is_card_present());
    
    // Test write protection (always false in our implementation)
    assert!(!driver.is_write_protected());
}