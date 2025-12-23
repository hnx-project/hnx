//! Test module for QoS functionality

extern crate alloc;

use crate::network::test::qos::*;
use crate::network::test::ipv4::Ipv4Header;

/// Test basic QoS rule creation and management
pub fn test_qos_rules() {
    crate::info!("network/test_qos: testing QoS rules");
    
    // Enable QoS
    enable();
    assert!(is_enabled());
    
    // Create a simple rule
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
    
    // List rules
    let rules = list_rules();
    assert_eq!(rules.len(), 1);
    
    // Remove rule
    remove_rule(rule_id).expect("Failed to remove QoS rule");
    
    // List rules again
    let rules = list_rules();
    assert_eq!(rules.len(), 0);
    
    // Disable QoS
    disable();
    assert!(!is_enabled());
    
    crate::info!("network/test_qos: QoS rules test passed");
}

/// Test packet classification
pub fn test_packet_classification() {
    crate::info!("network/test_qos: testing packet classification");
    
    // Enable QoS
    enable();
    
    // Create a rule for HTTP traffic
    create_rule(
        50, // priority
        QosAction::Allow { priority: 3 },
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
    
    // Create a rule for SSH traffic with higher priority
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
    
    // Create a rule for VoIP traffic with highest priority
    create_rule(
        10, // highest priority
        QosAction::Allow { priority: 0 },
        0, // any source
        0, // any source netmask
        0, // any destination
        0, // any destination netmask
        17, // UDP
        0, // any source port
        5060, // SIP destination port
        0, // min packet size
        0, // max packet size
        TrafficClass::Voice,
    ).expect("Failed to create QoS rule for VoIP");
    
    // Test packet classification for HTTP
    let http_header = Ipv4Header::new(
        0xC0A80001, // 192.168.0.1
        0xC0A80002, // 192.168.0.2
        6, // TCP
        100, // payload length
    );
    
    // Simulate TCP header for HTTP (port 80)
    let tcp_header_data = [
        0x00, 0x50, // Source port (random)
        0x00, 0x50, // Destination port (80 - HTTP)
        0x00, 0x00, 0x00, 0x00, // Sequence number
        0x00, 0x00, 0x00, 0x00, // Acknowledgment number
        0x50, 0x00, // Data offset + reserved + flags
        0x00, 0x00, // Window size
        0x00, 0x00, // Checksum
        0x00, 0x00, // Urgent pointer
    ];
    
    let action = classify_packet(&http_header, Some(&tcp_header_data), 120);
    match action {
        Some(QosAction::Allow { priority }) => {
            assert_eq!(priority, 3);
            crate::info!("network/test_qos: HTTP packet correctly classified with priority {}", priority);
        }
        _ => panic!("HTTP packet not classified correctly"),
    }
    
    // Test packet classification for SSH
    // Simulate TCP header for SSH (port 22)
    let ssh_header_data = [
        0x00, 0x50, // Source port (random)
        0x00, 0x16, // Destination port (22 - SSH)
        0x00, 0x00, 0x00, 0x00, // Sequence number
        0x00, 0x00, 0x00, 0x00, // Acknowledgment number
        0x50, 0x00, // Data offset + reserved + flags
        0x00, 0x00, // Window size
        0x00, 0x00, // Checksum
        0x00, 0x00, // Urgent pointer
    ];
    
    let action = classify_packet(&http_header, Some(&ssh_header_data), 120);
    match action {
        Some(QosAction::Allow { priority }) => {
            assert_eq!(priority, 1);
            crate::info!("network/test_qos: SSH packet correctly classified with priority {}", priority);
        }
        _ => panic!("SSH packet not classified correctly"),
    }
    
    // Test packet classification for VoIP
    let udp_header = Ipv4Header::new(
        0xC0A80001, // 192.168.0.1
        0xC0A80002, // 192.168.0.2
        17, // UDP
        100, // payload length
    );
    
    // Simulate UDP header for SIP (port 5060)
    let sip_header_data = [
        0x00, 0x50, // Source port (random)
        0x13, 0xC4, // Destination port (5060 - SIP)
        0x00, 0x00, // Length (to be filled)
        0x00, 0x00, // Checksum
    ];
    
    let action = classify_packet(&udp_header, Some(&sip_header_data), 120);
    match action {
        Some(QosAction::Allow { priority }) => {
            assert_eq!(priority, 0);
            crate::info!("network/test_qos: VoIP packet correctly classified with priority {}", priority);
        }
        _ => panic!("VoIP packet not classified correctly"),
    }
    
    // Test statistics
    let stats = get_stats();
    assert!(stats.classified_packets >= 3);
    assert!(stats.classified_bytes >= 360); // 3 packets * 120 bytes each
    
    crate::info!("network/test_qos: packet classification test passed");
    
    // Disable QoS
    disable();
}

/// Test traffic shaping
pub fn test_traffic_shaping() {
    crate::info!("network/test_qos: testing traffic shaping");
    
    // Enable QoS
    enable();
    
    // Create a rule with traffic shaping (1 Mbps)
    let rule_id = create_rule(
        25, // priority - lower than HTTP rule (50) but higher than SSH rule (25)
        QosAction::Shape { rate_bps: 1_000_000 }, // 1 Mbps
        0, // any source
        0, // any source netmask
        0, // any destination
        0, // any destination netmask
        6, // TCP
        0, // any source port
        80, // HTTP destination port
        0, // min packet size
        0, // max packet size
        TrafficClass::BestEffort,
    ).expect("Failed to create QoS rule with shaping");
    
    // Test packet classification for shaped traffic
    let http_header = Ipv4Header::new(
        0xC0A80001, // 192.168.0.1
        0xC0A80002, // 192.168.0.2
        6, // TCP
        1000, // payload length
    );
    
    // Simulate TCP header for HTTP (port 80)
    let tcp_header_data = [
        0x00, 0x50, // Source port (random)
        0x00, 0x50, // Destination port (80 - HTTP)
        0x00, 0x00, 0x00, 0x00, // Sequence number
        0x00, 0x00, 0x00, 0x00, // Acknowledgment number
        0x50, 0x00, // Data offset + reserved + flags
        0x00, 0x00, // Window size
        0x00, 0x00, // Checksum
        0x00, 0x00, // Urgent pointer
    ];
    
    // First packet should be allowed (within burst allowance)
    let action = classify_packet(&http_header, Some(&tcp_header_data), 1020);
    match action {
        Some(QosAction::Shape { .. }) => {
            crate::info!("network/test_qos: packet correctly shaped");
        }
        Some(ref a) => panic!("Packet classified with wrong action: {:?}", a),
        None => panic!("Packet not classified"),
    }
    
    // Clean up
    remove_rule(rule_id).expect("Failed to remove QoS rule");
    
    crate::info!("network/test_qos: traffic shaping test passed");
    
    // Disable QoS
    disable();
}

/// Test DSCP functionality
pub fn test_dscp() {
    crate::info!("network/test_qos: testing DSCP functionality");
    
    // Test DSCP to traffic class conversion
    assert_eq!(dscp_to_traffic_class(0), TrafficClass::BestEffort);
    assert_eq!(dscp_to_traffic_class(8), TrafficClass::Background);
    assert_eq!(dscp_to_traffic_class(10), TrafficClass::ExcellentEffort);
    assert_eq!(dscp_to_traffic_class(12), TrafficClass::ControlledLoad);
    assert_eq!(dscp_to_traffic_class(14), TrafficClass::Video);
    assert_eq!(dscp_to_traffic_class(16), TrafficClass::Voice);
    assert_eq!(dscp_to_traffic_class(24), TrafficClass::NetworkControl);
    assert_eq!(dscp_to_traffic_class(30), TrafficClass::Custom(30));
    
    // Test traffic class to DSCP conversion
    assert_eq!(traffic_class_to_dscp(TrafficClass::BestEffort), DscpValue::BestEffort);
    assert_eq!(traffic_class_to_dscp(TrafficClass::Background), DscpValue::Background);
    assert_eq!(traffic_class_to_dscp(TrafficClass::ExcellentEffort), DscpValue::ExcellentEffort);
    assert_eq!(traffic_class_to_dscp(TrafficClass::ControlledLoad), DscpValue::ControlledLoad);
    assert_eq!(traffic_class_to_dscp(TrafficClass::Video), DscpValue::Video);
    assert_eq!(traffic_class_to_dscp(TrafficClass::Voice), DscpValue::Voice);
    assert_eq!(traffic_class_to_dscp(TrafficClass::NetworkControl), DscpValue::NetworkControl);
    assert_eq!(traffic_class_to_dscp(TrafficClass::Custom(30)), DscpValue::Custom(30));
    
    // Test setting DSCP value in IP header
    let mut ip_header = Ipv4Header::new(
        0xC0A80001, // 192.168.0.1
        0xC0A80002, // 192.168.0.2
        6, // TCP
        100, // payload length
    );
    
    // Initially ToS should be 0
    assert_eq!(ip_header.type_of_service, 0);
    
    // Set DSCP value for voice traffic
    set_dscp_value(&mut ip_header, DscpValue::Voice);
    
    // Check that DSCP bits are set correctly (16 << 2 = 64)
    assert_eq!(ip_header.type_of_service & 0xFC, 64); // DSCP bits (first 6 bits)
    
    crate::info!("network/test_qos: DSCP functionality test passed");
}

/// Run all QoS tests
pub fn run_all_tests() {
    crate::info!("network/test_qos: running all QoS tests");
    
    test_qos_rules();
    test_packet_classification();
    test_traffic_shaping();
    test_dscp();
    
    crate::info!("network/test_qos: all tests passed");
}