//! Simple test program to verify QoS functionality

extern crate alloc;

use crate::network::test::qos::*;

/// Test QoS functionality
pub fn test_qos_functionality() {
    crate::info!("network/test_qos_simple: testing QoS functionality");
    
    // Enable QoS
    enable();
    assert!(is_enabled());
    
    // Create a simple QoS rule
    let rule_id = create_rule(
        100, // priority
        QosAction::Allow { priority: 5 },
        0xC0A80000, // 192.168.0.0
        0xFFFFFF00, // 255.255.255.0
        0xC0A80100, // 192.168.1.0
        0xFFFFFF00, // 255.255.255.0
        6, // TCP
        80, // source port
        8080, // destination port
        0, // min packet size
        0, // max packet size
        TrafficClass::Video,
    ).expect("Failed to create QoS rule");
    
    crate::info!("network/test_qos_simple: Created QoS rule with ID {}", rule_id);
    
    // List rules
    let rules = list_rules();
    crate::info!("network/test_qos_simple: Found {} QoS rules", rules.len());
    
    // Check statistics
    let stats = get_stats();
    crate::info!("network/test_qos_simple: Initial stats - classified: {}, dropped: {}", 
                stats.classified_packets, stats.dropped_packets);
    
    // Disable QoS
    disable();
    assert!(!is_enabled());
    
    crate::info!("network/test_qos_simple: QoS functionality test completed");
}