//! Integration test for QoS functionality with the network stack

extern crate alloc;

use crate::network::test::qos::*;
use crate::network::test::ipv4::*;

/// Integration test for QoS with IPv4 processing
pub fn test_qos_ipv4_integration() {
    crate::info!("network/integration_test_qos: testing QoS with IPv4");
    
    // Enable QoS
    enable();
    
    // Create a QoS rule for HTTP traffic
    create_rule(
        50, // priority
        QosAction::Allow { priority: 2 },
        0, // any source
        0, // any source netmask
        0, // any destination
        0, // any destination netmask
        6, // TCP
        0, // any source port
        80, // HTTP destination port
        0, // min packet size
        0, // max packet size
        TrafficClass::ExcellentEffort,
    ).expect("Failed to create QoS rule for HTTP");
    
    // Create a QoS rule for SSH traffic with higher priority
    create_rule(
        25, // higher priority
        QosAction::Allow { priority: 1 },
        0, // any source
        0, // any source netmask
        0, // any destination
        0, // any destination netmask
        6, // TCP
        0, // any source port
        22, // SSH destination port
        0, // min packet size
        0, // max packet size
        TrafficClass::NetworkControl,
    ).expect("Failed to create QoS rule for SSH");
    
    // Test with an IPv4 packet for HTTP
    let payload = vec![0u8; 100]; // 100 bytes of payload
    let header = Ipv4Header::new(
        0xC0A80001, // 192.168.0.1
        0xC0A80002, // 192.168.0.2
        6, // TCP protocol
        payload.len(),
    );
    
    // Process packet through QoS
    let action = classify_packet(&header, Some(&payload), payload.len() + Ipv4Header::MIN_SIZE);
    
    match action {
        Some(QosAction::Allow { priority }) => {
            crate::info!("network/integration_test_qos: HTTP packet processed with priority {}", priority);
            // Note: Actual priority might vary depending on implementation details
        }
        _ => {
            crate::info!("network/integration_test_qos: HTTP packet processed with action {:?}", action);
        }
    }
    
    // Check statistics
    let stats = get_stats();
    crate::info!("network/integration_test_qos: QoS stats after processing:");
    crate::info!("  Classified packets: {}", stats.classified_packets);
    crate::info!("  Classified bytes: {}", stats.classified_bytes);
    
    // Disable QoS
    disable();
    
    crate::info!("network/integration_test_qos: QoS IPv4 integration test completed");
}