//! Example program demonstrating firewall usage

extern crate alloc;

use crate::network::firewall::*;

/// Example showing how to configure basic firewall rules
pub fn firewall_configuration_example() {
    crate::info!("network/firewall_example: demonstrating firewall configuration");
    
    // Initialize the firewall
    init();
    
    // Enable the firewall
    enable();
    
    // Create a rule to allow SSH traffic (port 22)
    let ssh_rule_id = create_rule(
        100, // priority
        FirewallAction::Allow,
        FirewallDirection::Both,
        IpSpec::Any, // Any source IP
        IpSpec::Any, // Any destination IP
        ProtocolType::Tcp, // TCP protocol
        PortSpec::Any, // Any source port
        PortSpec::Single(22), // Destination port 22 (SSH)
    ).expect("Failed to create SSH rule");
    
    // Create a rule to block all traffic from a specific subnet
    let block_rule_id = create_rule(
        50, // Higher priority than SSH rule
        FirewallAction::Drop,
        FirewallDirection::Both,
        IpSpec::Subnet(0x0A000000u32, 0xFFFFFF00u32), // 10.0.0.0/24 subnet
        IpSpec::Any,
        ProtocolType::Any,
        PortSpec::Any,
        PortSpec::Any,
    ).expect("Failed to create block rule");
    
    // Create a rule to reject ICMP (ping) traffic
    let icmp_rule_id = create_rule(
        200, // Lower priority
        FirewallAction::Reject,
        FirewallDirection::Incoming,
        IpSpec::Any,
        IpSpec::Any,
        ProtocolType::Icmp,
        PortSpec::Any,
        PortSpec::Any,
    ).expect("Failed to create ICMP rule");
    
    // List all rules (minimal output to avoid console spam)
    let rules = list_rules();
    if rules.len() > 0 {
        crate::info!("network/firewall_example: created {} rules", rules.len());
    }
    
    // Show firewall statistics
    let stats = get_stats();
    if stats.allowed_packets > 0 || stats.dropped_packets > 0 || stats.rejected_packets > 0 {
        crate::info!("network/firewall_example: processed {} packets", 
                    stats.allowed_packets + stats.dropped_packets + stats.rejected_packets);
    }
    
    // Reset statistics
    reset_stats();
    
    // Remove the SSH rule
    remove_rule(ssh_rule_id).expect("Failed to remove SSH rule");
    
    crate::info!("network/firewall_example: firewall configuration demonstration complete");
}