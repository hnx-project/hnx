//! Tests for the firewall module

extern crate alloc;

use crate::network::test::firewall::*;
use crate::network::test::ipv4::Ipv4Header;

/// Test basic firewall functionality
pub fn test_firewall_basic() {
    crate::info!("network/test_firewall: testing basic firewall functionality");
    
    // Initialize firewall
    init();
    
    // Check that firewall is initially disabled
    assert!(!is_enabled());
    
    // Enable firewall
    enable();
    assert!(is_enabled());
    
    // Disable firewall
    disable();
    assert!(!is_enabled());
    
    // Re-enable firewall
    enable();
    assert!(is_enabled());
    
    crate::info!("network/test_firewall: basic firewall functionality test passed");
}

/// Test firewall rule creation and management
pub fn test_firewall_rules() {
    crate::info!("network/test_firewall: testing firewall rule management");
    
    // Initialize firewall
    init();
    enable();
    
    // Create a simple allow rule
    let rule_id = create_rule(
        100, // priority
        FirewallAction::Allow,
        FirewallDirection::Both,
        IpSpec::Any,
        IpSpec::Any,
        ProtocolType::Any,
        PortSpec::Any,
        PortSpec::Any,
    ).expect("Failed to create rule");
    
    // Check that we can list rules
    let rules = list_rules();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].id, rule_id);
    
    // Remove the rule
    remove_rule(rule_id).expect("Failed to remove rule");
    
    // Check that rules list is now empty
    let rules = list_rules();
    assert_eq!(rules.len(), 0);
    
    // Try to remove a non-existent rule
    assert!(remove_rule(999).is_err());
    
    crate::info!("network/test_firewall: firewall rule management test passed");
}

/// Test firewall packet processing
pub fn test_firewall_processing() {
    crate::info!("network/test_firewall: testing firewall packet processing");
    
    // Initialize firewall
    init();
    enable();
    
    // Reset statistics
    reset_stats();
    
    // Create a simple allow rule
    create_rule(
        100, // priority
        FirewallAction::Allow,
        FirewallDirection::Both,
        IpSpec::Any,
        IpSpec::Any,
        ProtocolType::Any,
        PortSpec::Any,
        PortSpec::Any,
    ).expect("Failed to create rule");
    
    // Create a test IPv4 header
    let ip_header = Ipv4Header::new(
        0x0A000001u32.to_be(), // 10.0.0.1
        0x0A000002u32.to_be(), // 10.0.0.2
        crate::network::ipv4::protocol::TCP,
        100, // payload length
    );
    
    // Process an incoming packet
    let action = process_incoming_packet(&ip_header, None)
        .expect("Failed to process incoming packet");
    
    // Should be allowed by our rule
    assert_eq!(action, FirewallAction::Allow);
    
    // Check statistics
    let stats = get_stats();
    assert_eq!(stats.allowed_packets, 1);
    assert_eq!(stats.dropped_packets, 0);
    assert_eq!(stats.rejected_packets, 0);
    
    // Create a drop rule with higher priority
    create_rule(
        50, // higher priority
        FirewallAction::Drop,
        FirewallDirection::Incoming,
        IpSpec::Single(0x0A000001u32.to_be()), // 10.0.0.1
        IpSpec::Any,
        ProtocolType::Tcp,
        PortSpec::Any,
        PortSpec::Any,
    ).expect("Failed to create drop rule");
    
    // Process another incoming packet from the same source
    let action = process_incoming_packet(&ip_header, None)
        .expect("Failed to process incoming packet");
    
    // Should be dropped by our higher priority rule
    assert_eq!(action, FirewallAction::Drop);
    
    // Check updated statistics
    let stats = get_stats();
    assert_eq!(stats.allowed_packets, 1);
    assert_eq!(stats.dropped_packets, 1);
    assert_eq!(stats.rejected_packets, 0);
    
    crate::info!("network/test_firewall: firewall packet processing test passed");
}

/// Test IP matching functionality
pub fn test_ip_matching() {
    crate::info!("network/test_firewall: testing IP matching");
    
    // Test exact IP match
    assert!(ip_matches(0x0A000001u32, &IpSpec::Single(0x0A000001u32)));
    assert!(!ip_matches(0x0A000001u32, &IpSpec::Single(0x0A000002u32)));
    
    // Test "any" match
    assert!(ip_matches(0x0A000001u32, &IpSpec::Any));
    
    // Test subnet match
    // 10.0.0.5 & 255.255.255.0 = 10.0.0.0
    // 10.0.0.100 & 255.255.255.0 = 10.0.0.0
    assert!(ip_matches(
        0x0A000005u32,
        &IpSpec::Subnet(0x0A000064u32, 0xFFFFFF00u32) // 10.0.0.100/24
    ));
    
    // 10.0.0.5 & 255.255.255.0 = 10.0.0.0
    // 10.0.1.100 & 255.255.255.0 = 10.0.1.0
    assert!(!ip_matches(
        0x0A000005u32,
        &IpSpec::Subnet(0x0A000164u32, 0xFFFFFF00u32) // 10.0.1.100/24
    ));
    
    crate::info!("network/test_firewall: IP matching test passed");
}

/// Test port matching functionality
pub fn test_port_matching() {
    crate::info!("network/test_firewall: testing port matching");
    
    // Test exact port match
    assert!(port_matches(80, &PortSpec::Single(80)));
    assert!(!port_matches(80, &PortSpec::Single(81)));
    
    // Test "any" match
    assert!(port_matches(80, &PortSpec::Any));
    
    // Test range match
    assert!(port_matches(80, &PortSpec::Range(70, 90)));
    assert!(port_matches(70, &PortSpec::Range(70, 90)));
    assert!(port_matches(90, &PortSpec::Range(70, 90)));
    assert!(!port_matches(69, &PortSpec::Range(70, 90)));
    assert!(!port_matches(91, &PortSpec::Range(70, 90)));
    
    crate::info!("network/test_firewall: port matching test passed");
}

/// Run all firewall tests
pub fn run_all_tests() {
    // Temporarily disable logging during tests to prevent console spam
    // We'll restore it at the end
    
    test_firewall_basic();
    test_firewall_rules();
    test_firewall_processing();
    test_ip_matching();
    test_port_matching();
    
    crate::info!("network/test_firewall: all tests passed");
}