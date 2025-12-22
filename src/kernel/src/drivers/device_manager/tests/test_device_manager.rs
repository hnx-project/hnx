//! Device manager tests

use super::*;

#[test]
fn test_basic_device_registration() {
    let mut dm = DeviceManager::new();
    
    // Create a basic device
    let mmio_regions = alloc::vec::Vec::new();
    let irqs = alloc::vec![5];
    let mut device_info = DeviceInfo::new(
        DeviceType::Storage,
        "Test Storage Device".into(),
        mmio_regions,
        irqs,
    );
    device_info.id = DeviceId(1);
    
    // Register the device
    let device_id = DeviceId(1);
    dm.devices.insert(device_id, device_info);
    
    // Verify the device is registered
    assert!(dm.devices.contains_key(&device_id));
}

#[test]
fn test_driver_registration() {
    let mut dm = DeviceManager::new();
    
    // Create a driver registration
    let reg = DeviceRegistration {
        device_type: DeviceType::Network,
        vendor_id: 0x1234,
        device_id: 0x5678,
        name: [0; 32],
    };
    
    let endpoint_id = EndpointId(100);
    
    // Register the driver
    let result = dm.register_driver(reg, endpoint_id);
    assert!(result.is_ok());
    
    let driver_id = result.unwrap();
    
    // Verify the driver is registered
    assert!(dm.drivers.contains_key(&driver_id));
    
    let driver_info = &dm.drivers[&driver_id];
    assert_eq!(driver_info.endpoint, endpoint_id);
    assert_eq!(driver_info.supported_devices.len(), 1);
    assert_eq!(driver_info.supported_devices[0], DeviceType::Network);
}

#[test]
fn test_irq_request_and_forwarding() {
    let mut dm = DeviceManager::new();
    
    // Register a driver first
    let reg = DeviceRegistration {
        device_type: DeviceType::Block,
        vendor_id: 0xabcd,
        device_id: 0xef01,
        name: [0; 32],
    };
    
    let driver_endpoint = EndpointId(200);
    let driver_id = dm.register_driver(reg, driver_endpoint).unwrap();
    
    // Request an IRQ
    let irq_req = IrqRequest {
        irq_number: 32,
        driver_epid: driver_endpoint,
    };
    
    let result = dm.request_irq(irq_req);
    assert!(result.is_ok());
    
    // Verify IRQ is routed correctly
    assert!(dm.irq_routing.contains_key(&32));
    assert_eq!(dm.irq_routing[&32], driver_endpoint);
    
    // Test IRQ forwarding logic
    assert!(dm.is_user_space_irq(32));
    assert!(!dm.is_user_space_irq(33)); // Not registered IRQ
}

#[test]
fn test_mmio_mapping_request() {
    let mut dm = DeviceManager::new();
    
    let mmio_req = MmioRequest {
        physical_address: 0x40000000,
        size: 0x1000,
        driver_epid: EndpointId(300),
    };
    
    let result = dm.request_mmio_mapping(mmio_req);
    assert!(result.is_ok());
    
    let capability = result.unwrap();
    assert_ne!(capability.id(), 0);
    
    // Verify the MMIO region is stored
    assert!(dm.mmio_regions.contains_key(&0x40000000));
}

#[test]
fn test_hotplug_device_registration() {
    let mut dm = DeviceManager::new();
    
    // Create a hotplug-capable device
    let mmio_regions = alloc::vec::Vec::new();
    let irqs = alloc::vec![10];
    let device_info = DeviceInfo::new_hotplug(
        DeviceType::Storage,
        "Test SD Card".into(),
        mmio_regions,
        irqs,
    );
    
    // Register the hotplug device
    let device_id = dm.register_hotplug_device(device_info);
    
    // Verify the device is registered but not present
    assert!(!dm.is_device_present(device_id));
    assert!(dm.devices.contains_key(&device_id));
    
    // Check that the device is marked as hotplug capable
    let device = &dm.devices[&device_id];
    assert!(device.hotplug_capable);
}

#[test]
fn test_hotplug_events() {
    use core::sync::atomic::{AtomicU32, Ordering};
    
    static EVENTS_RECEIVED: AtomicU32 = AtomicU32::new(0);
    static LAST_EVENT_TYPE: AtomicU32 = AtomicU32::new(0); // 0 = none, 1 = added, 2 = removed
    
    fn test_callback(event: HotplugEvent) {
        EVENTS_RECEIVED.fetch_add(1, Ordering::SeqCst);
        match event {
            HotplugEvent::DeviceAdded(_) => LAST_EVENT_TYPE.store(1, Ordering::SeqCst),
            HotplugEvent::DeviceRemoved(_) => LAST_EVENT_TYPE.store(2, Ordering::SeqCst),
        }
    }
    
    let mut dm = DeviceManager::new();
    dm.register_hotplug_callback(test_callback);
    
    // Create a hotplug-capable device
    let mmio_regions = alloc::vec::Vec::new();
    let irqs = alloc::vec![11];
    let device_info = DeviceInfo::new_hotplug(
        DeviceType::Block,
        "Test MMC Card".into(),
        mmio_regions,
        irqs,
    );
    
    let device_id = dm.register_hotplug_device(device_info);
    
    // Initially no events should be received
    assert_eq!(EVENTS_RECEIVED.load(Ordering::SeqCst), 0);
    
    // Simulate device insertion
    dm.detect_hotplug_event(device_id, true);
    
    // Verify callback was called for addition
    assert_eq!(EVENTS_RECEIVED.load(Ordering::SeqCst), 1);
    assert_eq!(LAST_EVENT_TYPE.load(Ordering::SeqCst), 1);
    
    // Verify device is now present
    assert!(dm.is_device_present(device_id));
    
    // Simulate device removal
    dm.detect_hotplug_event(device_id, false);
    
    // Verify callback was called for removal
    assert_eq!(EVENTS_RECEIVED.load(Ordering::SeqCst), 2);
    assert_eq!(LAST_EVENT_TYPE.load(Ordering::SeqCst), 2);
    
    // Verify device is no longer present
    assert!(!dm.is_device_present(device_id));
}

#[test]
fn test_block_device_support() {
    let mut dm = DeviceManager::new();
    
    // Create a block device
    let mmio_regions = alloc::vec![
        MmioRegion {
            physical_address: 0x50000000,
            size: 0x1000,
            capability: Capability::new_mmio(0x50000000, 0x1000),
        }
    ];
    let irqs = alloc::vec![45, 46];
    let mut device_info = DeviceInfo::new(
        DeviceType::Block,
        "NVMe SSD".into(),
        mmio_regions,
        irqs,
    );
    device_info.id = DeviceId(100);
    
    // Register the block device
    dm.devices.insert(device_info.id, device_info.clone());
    
    // Verify block device properties
    let registered_device = &dm.devices[&DeviceId(100)];
    assert_eq!(registered_device.device_type, DeviceType::Block);
    assert_eq!(registered_device.name, "NVMe SSD");
    assert_eq!(registered_device.mmio_regions.len(), 1);
    assert_eq!(registered_device.irqs.len(), 2);
    assert_eq!(registered_device.irqs[0], 45);
    assert_eq!(registered_device.irqs[1], 46);
    
    // Verify MMIO regions are stored
    assert!(dm.mmio_regions.contains_key(&0x50000000));
}

#[test]
fn test_network_device_enhancements() {
    let mut dm = DeviceManager::new();
    
    // Create a network device with multiple MMIO regions
    let mmio_regions = alloc::vec![
        MmioRegion {
            physical_address: 0x60000000,
            size: 0x10000, // Large region for registers
            capability: Capability::new_mmio(0x60000000, 0x10000),
        },
        MmioRegion {
            physical_address: 0x60010000,
            size: 0x2000, // Smaller region for buffers
            capability: Capability::new_mmio(0x60010000, 0x2000),
        }
    ];
    let irqs = alloc::vec![50, 51, 52]; // Multiple IRQs for different functions
    let mut device_info = DeviceInfo::new(
        DeviceType::Network,
        "Ethernet Controller".into(),
        mmio_regions,
        irqs,
    );
    device_info.id = DeviceId(200);
    
    // Register the network device
    dm.devices.insert(device_info.id, device_info.clone());
    
    // Verify network device properties
    let registered_device = &dm.devices[&DeviceId(200)];
    assert_eq!(registered_device.device_type, DeviceType::Network);
    assert_eq!(registered_device.name, "Ethernet Controller");
    assert_eq!(registered_device.mmio_regions.len(), 2);
    assert_eq!(registered_device.irqs.len(), 3);
    
    // Verify all MMIO regions are stored
    assert!(dm.mmio_regions.contains_key(&0x60000000));
    assert!(dm.mmio_regions.contains_key(&0x60010000));
    
    // Test DMA buffer allocation simulation
    let dma_cap = Capability::new_dma_buffer(0x70000000, 0xFFFF0000, 0x4000);
    assert_ne!(dma_cap.id(), 0);
    
    match dma_cap.cap_type {
        CapabilityType::DmaBuffer { physical_address, virtual_address, size } => {
            assert_eq!(physical_address, 0x70000000);
            assert_eq!(virtual_address, 0xFFFF0000);
            assert_eq!(size, 0x4000);
        }
        _ => panic!("Expected DMA buffer capability"),
    }
}

#[test]
fn test_multiple_hotplug_callbacks() {
    use core::sync::atomic::{AtomicU32, Ordering};
    
    static CALLBACK1_COUNT: AtomicU32 = AtomicU32::new(0);
    static CALLBACK2_COUNT: AtomicU32 = AtomicU32::new(0);
    
    fn callback1(_event: HotplugEvent) {
        CALLBACK1_COUNT.fetch_add(1, Ordering::SeqCst);
    }
    
    fn callback2(_event: HotplugEvent) {
        CALLBACK2_COUNT.fetch_add(1, Ordering::SeqCst);
    }
    
    let mut dm = DeviceManager::new();
    dm.register_hotplug_callback(callback1);
    dm.register_hotplug_callback(callback2);
    
    // Create a hotplug device
    let mmio_regions = alloc::vec::Vec::new();
    let irqs = alloc::vec![15];
    let device_info = DeviceInfo::new_hotplug(
        DeviceType::Usb,
        "USB Device".into(),
        mmio_regions,
        irqs,
    );
    
    let device_id = dm.register_hotplug_device(device_info);
    
    // Trigger hotplug event
    dm.detect_hotplug_event(device_id, true);
    
    // Both callbacks should be called
    assert_eq!(CALLBACK1_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(CALLBACK2_COUNT.load(Ordering::SeqCst), 1);
}

#[test]
fn test_device_capability_generation() {
    // Test that capabilities have unique IDs
    let cap1 = Capability::new_mmio(0x10000000, 0x1000);
    let cap2 = Capability::new_mmio(0x20000000, 0x1000);
    
    assert_ne!(cap1.id(), cap2.id());
    assert_ne!(cap1.id(), 0);
    assert_ne!(cap2.id(), 0);
    
    // Test DMA capability generation
    let dma_cap1 = Capability::new_dma_buffer(0x30000000, 0xFFFF3000, 0x1000);
    let dma_cap2 = Capability::new_dma_buffer(0x40000000, 0xFFFF4000, 0x1000);
    
    assert_ne!(dma_cap1.id(), dma_cap2.id());
    assert_ne!(dma_cap1.id(), 0);
    assert_ne!(dma_cap2.id(), 0);
    assert_ne!(dma_cap1.id(), cap1.id());
}