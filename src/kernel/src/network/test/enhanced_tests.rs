//! Enhanced network testing framework with IPv6 and advanced feature support
//!
//! This module extends the existing network testing framework to support testing
//! of IPv6 functionality, enhanced firewall features, and advanced QoS mechanisms.

extern crate alloc;

use crate::network::test::*;

/// Test IPv6 basic functionality
pub fn test_ipv6_basic() -> Result<(), &'static str> {
    crate::info!("network/test_framework: testing IPv6 basic functionality");
    
    // Test IPv6 header creation
    let source_addr: [u8; 16] = [0x20, 0x01, 0xd, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
    let dest_addr: [u8; 16] = [0x20, 0x01, 0xd, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2];
    
    let header = crate::network::ipv6::Ipv6Header::new(
        source_addr,
        dest_addr,
        crate::network::ipv6::next_header::TCP,
        100, // payload length
    );
    
    // Verify header fields
    if header.version() != 6 {
        return Err("IPv6 version incorrect");
    }
    
    if header.traffic_class() != 0 {
        return Err("IPv6 traffic class incorrect");
    }
    
    if header.flow_label() != 0 {
        return Err("IPv6 flow label incorrect");
    }
    
    if header.payload_length() != 100 {
        return Err("IPv6 payload length incorrect");
    }
    
    if header.next_header != crate::network::ipv6::next_header::TCP {
        return Err("IPv6 next header incorrect");
    }
    
    if header.hop_limit != 64 {
        return Err("IPv6 hop limit incorrect");
    }
    
    if header.source_addr != source_addr {
        return Err("IPv6 source address incorrect");
    }
    
    if header.destination_addr != dest_addr {
        return Err("IPv6 destination address incorrect");
    }
    
    crate::info!("network/test_framework: IPv6 basic functionality test passed");
    Ok(())
}

/// Test IPv6 packet creation and parsing
pub fn test_ipv6_packet() -> Result<(), &'static str> {
    crate::info!("network/test_framework: testing IPv6 packet creation and parsing");
    
    // Create test data
    let source_addr: [u8; 16] = [0x20, 0x01, 0xd, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
    let dest_addr: [u8; 16] = [0x20, 0x01, 0xd, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2];
    let payload = alloc::vec![1, 2, 3, 4, 5];
    
    // Create IPv6 header
    let header = crate::network::ipv6::Ipv6Header::new(
        source_addr,
        dest_addr,
        crate::network::ipv6::next_header::TCP,
        payload.len(),
    );
    
    // Create IPv6 packet
    let packet = crate::network::ipv6::Ipv6Packet::new(header, payload.clone());
    
    // Convert to bytes
    let bytes = packet.to_bytes();
    
    // Parse back from bytes
    let parsed_packet = crate::network::ipv6::Ipv6Packet::from_bytes(&bytes)
        .map_err(|_| "Failed to parse IPv6 packet")?;
    
    // Verify parsed packet
    if parsed_packet.header.version() != 6 {
        return Err("Parsed IPv6 version incorrect");
    }
    
    if parsed_packet.payload != payload {
        return Err("Parsed IPv6 payload incorrect");
    }
    
    crate::info!("network/test_framework: IPv6 packet creation and parsing test passed");
    Ok(())
}

/// Test enhanced firewall functionality with IPv6 support
pub fn test_enhanced_firewall() -> Result<(), &'static str> {
    crate::info!("network/test_framework: testing enhanced firewall functionality");
    
    // Enable the enhanced firewall
    crate::network::firewall_enhanced::enable();
    
    // Test IPv6 rule creation
    let source_ipv6: [u8; 16] = [0x20, 0x01, 0xd, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let dest_ipv6: [u8; 16] = [0xff, 0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
    
    let rule_id = crate::network::firewall_enhanced::create_rule_v6(
        100, // priority
        crate::network::firewall::FirewallAction::Allow,
        crate::network::firewall::FirewallDirection::Incoming,
        crate::network::firewall_enhanced::Ipv6Spec::Subnet(source_ipv6, 64),
        crate::network::firewall_enhanced::Ipv6Spec::Single(dest_ipv6),
        crate::network::firewall::ProtocolType::Tcp,
        crate::network::firewall::PortSpec::Any,
        crate::network::firewall::PortSpec::Single(80),
    ).map_err(|_| "Failed to create IPv6 firewall rule")?;
    
    // Verify rule was created
    let rules = crate::network::firewall_enhanced::list_rules_v6();
    if rules.is_empty() {
        return Err("No IPv6 firewall rules found");
    }
    
    if rules[0].id != rule_id {
        return Err("IPv6 firewall rule ID mismatch");
    }
    
    // Test IPv6 matching function
    let test_addr: [u8; 16] = [0x20, 0x01, 0xd, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5];
    let spec = crate::network::firewall_enhanced::Ipv6Spec::Subnet(source_ipv6, 64);
    
    if !crate::network::firewall_enhanced::ipv6_matches(test_addr, &spec) {
        return Err("IPv6 address matching failed");
    }
    
    // Clean up
    crate::network::firewall_enhanced::remove_rule_v6(rule_id)
        .map_err(|_| "Failed to remove IPv6 firewall rule")?;
    
    crate::info!("network/test_framework: enhanced firewall functionality test passed");
    Ok(())
}

/// Test enhanced QoS functionality with IPv6 support
pub fn test_enhanced_qos() -> Result<(), &'static str> {
    crate::info!("network/test_framework: testing enhanced QoS functionality");
    
    // Enable the enhanced QoS system
    crate::network::qos_enhanced::enable();
    
    // Test IPv6 QoS rule creation
    let source_ipv6: [u8; 16] = [0x20, 0x01, 0xd, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let dest_ipv6: [u8; 16] = [0xff, 0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
    
    let rule_id = crate::network::qos_enhanced::create_rule_v6(
        100, // priority
        crate::network::qos::QosAction::Allow { priority: 1 },
        source_ipv6,
        64, // source prefix length
        dest_ipv6,
        128, // dest prefix length
        crate::network::ipv6::next_header::TCP,
        1024, // source port
        80,   // dest port
        0,    // min packet size
        1500, // max packet size
        crate::network::qos::TrafficClass::Video,
    ).map_err(|_| "Failed to create IPv6 QoS rule")?;
    
    // Verify rule was created
    let rules = crate::network::qos_enhanced::list_rules_v6();
    if rules.is_empty() {
        return Err("No IPv6 QoS rules found");
    }
    
    if rules[0].id != rule_id {
        return Err("IPv6 QoS rule ID mismatch");
    }
    
    // Test IPv6 matching function
    let test_addr: [u8; 16] = [0x20, 0x01, 0xd, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5];
    if !crate::network::qos_enhanced::ipv6_matches(test_addr, source_ipv6, 64) {
        return Err("IPv6 address matching failed");
    }
    
    // Clean up
    crate::network::qos_enhanced::remove_rule_v6(rule_id)
        .map_err(|_| "Failed to remove IPv6 QoS rule")?;
    
    crate::info!("network/test_framework: enhanced QoS functionality test passed");
    Ok(())
}

/// Test TCP congestion control algorithms
pub fn test_tcp_congestion_control() -> Result<(), &'static str> {
    crate::info!("network/test_framework: testing TCP congestion control algorithms");
    
    // Test Tahoe congestion control
    let mut cc_tahoe = crate::network::tcp_congestion::CongestionControl::new(
        crate::network::tcp_congestion::CongestionControlAlgorithm::Tahoe
    );
    
    // Test initial state
    if cc_tahoe.cwnd != 1 {
        return Err("Tahoe initial cwnd incorrect");
    }
    
    // Test slow start
    cc_tahoe.on_new_ack(1);
    if cc_tahoe.cwnd != 2 {
        return Err("Tahoe slow start incorrect");
    }
    
    // Test congestion avoidance
    cc_tahoe.ssthresh = 2;
    cc_tahoe.on_new_ack(1);
    if cc_tahoe.cwnd != 3 {
        return Err("Tahoe congestion avoidance incorrect");
    }
    
    // Test timeout
    cc_tahoe.on_timeout();
    if cc_tahoe.cwnd != 1 {
        return Err("Tahoe timeout handling incorrect");
    }
    
    // Test Reno congestion control
    let mut cc_reno = crate::network::tcp_congestion::CongestionControl::new(
        crate::network::tcp_congestion::CongestionControlAlgorithm::Reno
    );
    
    // Test fast retransmit
    cc_reno.on_dup_ack(); // dup_acks = 1
    cc_reno.on_dup_ack(); // dup_acks = 2
    cc_reno.on_dup_ack(); // dup_acks = 3, should trigger fast retransmit
    if cc_reno.dup_acks != 3 {
        return Err("Reno duplicate ACK counting incorrect");
    }
    
    crate::info!("network/test_framework: TCP congestion control algorithms test passed");
    Ok(())
}

/// Test HTB scheduler functionality
pub fn test_htb_scheduler() -> Result<(), &'static str> {
    crate::info!("network/test_framework: testing HTB scheduler functionality");
    
    // Initialize HTB scheduler
    crate::network::qos_enhanced::htb_scheduler::init();
    
    // Create a root class
    let root_class = crate::network::qos_enhanced::htb_scheduler::HtbClass {
        class_id: 1,
        parent_id: None,
        rate: 1000000, // 1 MBps
        ceil: 2000000, // 2 MBps
        buffer: 1000,
        cbuffer: 2000,
        quantum: 1500,
        level: 1,
        prio: 0,
        children: alloc::vec![],
    };
    
    // Add the root class
    crate::network::qos_enhanced::htb_scheduler::add_class(root_class)
        .map_err(|_| "Failed to add HTB root class")?;
    
    // Create a leaf class
    let leaf_class = crate::network::qos_enhanced::htb_scheduler::HtbClass {
        class_id: 2,
        parent_id: Some(1),
        rate: 500000, // 500 KBps
        ceil: 1000000, // 1 MBps
        buffer: 500,
        cbuffer: 1000,
        quantum: 1500,
        level: 0,
        prio: 1,
        children: alloc::vec![],
    };
    
    // Add the leaf class
    crate::network::qos_enhanced::htb_scheduler::add_class(leaf_class)
        .map_err(|_| "Failed to add HTB leaf class")?;
    
    // Verify classes were added
    let classes = crate::network::qos_enhanced::htb_scheduler::list_classes();
    if classes.len() != 2 {
        return Err("HTB class count incorrect");
    }
    
    // Verify root class
    let root = crate::network::qos_enhanced::htb_scheduler::get_class(1)
        .ok_or("Root class not found")?;
    if root.class_id != 1 {
        return Err("Root class ID incorrect");
    }
    
    // Verify leaf class
    let leaf = crate::network::qos_enhanced::htb_scheduler::get_class(2)
        .ok_or("Leaf class not found")?;
    if leaf.parent_id != Some(1) {
        return Err("Leaf class parent ID incorrect");
    }
    
    // Clean up
    crate::network::qos_enhanced::htb_scheduler::remove_class(2)
        .map_err(|_| "Failed to remove HTB leaf class")?;
    crate::network::qos_enhanced::htb_scheduler::remove_class(1)
        .map_err(|_| "Failed to remove HTB root class")?;
    
    crate::info!("network/test_framework: HTB scheduler functionality test passed");
    Ok(())
}

/// Test SFQ scheduler functionality
pub fn test_sfq_scheduler() -> Result<(), &'static str> {
    crate::info!("network/test_framework: testing SFQ scheduler functionality");
    
    // Initialize SFQ scheduler
    crate::network::qos_enhanced::sfq_scheduler::init();
    
    // Create test flows
    let flow1 = crate::network::qos_enhanced::sfq_scheduler::FlowId {
        src_ip: [0; 16],
        dst_ip: [0; 16],
        src_port: 1000,
        dst_port: 2000,
        protocol: 6, // TCP
    };
    
    let flow2 = crate::network::qos_enhanced::sfq_scheduler::FlowId {
        src_ip: [0; 16],
        dst_ip: [0; 16],
        src_port: 3000,
        dst_port: 4000,
        protocol: 17, // UDP
    };
    
    // Create test packets
    let packet1 = alloc::vec![1, 2, 3, 4, 5];
    let packet2 = alloc::vec![6, 7, 8, 9, 10];
    
    // Enqueue packets
    crate::network::qos_enhanced::sfq_scheduler::enqueue_packet(flow1, packet1.clone())
        .map_err(|_| "Failed to enqueue packet 1")?;
    crate::network::qos_enhanced::sfq_scheduler::enqueue_packet(flow2, packet2.clone())
        .map_err(|_| "Failed to enqueue packet 2")?;
    
    // Dequeue packets
    let dequeued1 = crate::network::qos_enhanced::sfq_scheduler::dequeue_packet()
        .ok_or("Failed to dequeue packet 1")?;
    let dequeued2 = crate::network::qos_enhanced::sfq_scheduler::dequeue_packet()
        .ok_or("Failed to dequeue packet 2")?;
    
    // Verify dequeued packets
    if dequeued1.1 != packet1 {
        return Err("Dequeued packet 1 content incorrect");
    }
    
    if dequeued2.1 != packet2 {
        return Err("Dequeued packet 2 content incorrect");
    }
    
    crate::info!("network/test_framework: SFQ scheduler functionality test passed");
    Ok(())
}

/// Run all enhanced network tests
pub fn run_all_enhanced_network_tests() -> Result<(), &'static str> {
    crate::info!("network/test_framework: starting all enhanced network tests");
    
    // Run IPv6 tests
    test_ipv6_basic()?;
    test_ipv6_packet()?;
    
    // Run enhanced firewall tests
    test_enhanced_firewall()?;
    
    // Run enhanced QoS tests
    test_enhanced_qos()?;
    
    // Run TCP congestion control tests
    test_tcp_congestion_control()?;
    
    // Run HTB scheduler tests
    test_htb_scheduler()?;
    
    // Run SFQ scheduler tests
    test_sfq_scheduler()?;
    
    crate::info!("network/test_framework: all enhanced network tests completed successfully");
    Ok(())
}