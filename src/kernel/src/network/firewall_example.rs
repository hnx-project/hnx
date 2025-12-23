//! Example program demonstrating firewall usage

extern crate alloc;

use crate::network::firewall::*;
use crate::network::firewall_enhanced::{FirewallDirection, IpSpec, PortSpec, ProtocolType};

/// Example showing how to configure basic firewall rules
pub fn firewall_configuration_example() {
    crate::info!("network/firewall_example: demonstrating firewall configuration");
    
    // Initialize the firewall
    init();
    
    // Enable the firewall
    // Note: The enable function is not implemented in the current firewall module
    
    // Create a rule to allow SSH traffic (port 22)
    let ssh_rule_id = add_rule(FirewallRule {
        id: 0,
        action: FirewallAction::Allow,
        direction: FirewallDirection::Both,
        src_ip: IpSpec::Any, // Any source IP
        dst_ip: IpSpec::Any, // Any destination IP
        src_ipv6: [0; 16],
        dst_ipv6: [0; 16],
        src_prefix_len: 0,
        dst_prefix_len: 0,
        protocol: ProtocolType::Tcp, // TCP protocol
        src_port: PortSpec::Any, // Any source port
        dst_port: PortSpec::Single(22), // Destination port 22 (SSH)
        priority: 100,
    }).expect("Failed to create SSH rule");
    
    // Create a rule to block all traffic from a specific subnet
    let block_rule_id = add_rule(FirewallRule {
        id: 0,
        action: FirewallAction::Drop,
        direction: FirewallDirection::Both,
        src_ip: IpSpec::Subnet(0x0A000000u32, 0xFFFFFF00u32), // 10.0.0.0/24 subnet
        dst_ip: IpSpec::Any,
        src_ipv6: [0; 16],
        dst_ipv6: [0; 16],
        src_prefix_len: 0,
        dst_prefix_len: 0,
        protocol: ProtocolType::Any,
        src_port: PortSpec::Any,
        dst_port: PortSpec::Any,
        priority: 50, // Higher priority than SSH rule
    }).expect("Failed to create block rule");
    
    // Create a rule to reject ICMP (ping) traffic
    let icmp_rule_id = add_rule(FirewallRule {
        id: 0,
        action: FirewallAction::Reject,
        direction: FirewallDirection::Incoming,
        src_ip: IpSpec::Any,
        dst_ip: IpSpec::Any,
        src_ipv6: [0; 16],
        dst_ipv6: [0; 16],
        src_prefix_len: 0,
        dst_prefix_len: 0,
        protocol: ProtocolType::Icmp,
        src_port: PortSpec::Any,
        dst_port: PortSpec::Any,
        priority: 200, // Lower priority
    }).expect("Failed to create ICMP rule");
    
    // List all rules (minimal output to avoid console spam)
    let rules = get_rules();
    if rules.len() > 0 {
        crate::info!("network/firewall_example: created {} rules", rules.len());
    }
    
    // Show firewall statistics
    let stats = get_stats();
    if stats.base_stats.allowed > 0 || stats.base_stats.dropped > 0 || stats.base_stats.matched > 0 {
        crate::info!("network/firewall_example: processed {} packets (allowed: {}, dropped: {}, matched: {})", 
                    stats.base_stats.allowed + stats.base_stats.dropped + stats.base_stats.matched,
                    stats.base_stats.allowed, stats.base_stats.dropped, stats.base_stats.matched);
    }
    
    // Reset statistics
    reset_stats();
    
    // Remove the SSH rule
    remove_rule(ssh_rule_id).expect("Failed to remove SSH rule");
    
    crate::info!("network/firewall_example: firewall configuration demonstration complete");
}