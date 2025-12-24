//! Example program demonstrating QoS configuration and usage

extern crate alloc;

use crate::network::qos::*;

/// Example showing how to configure QoS rules
pub fn configure_qos_example() {
    crate::info!("network/qos_example: configuring QoS rules");
    
    // Enable QoS system
    enable();
    
    // Create a simple rule for VoIP traffic (highest priority)
    let voip_rule_id = create_rule(
        10, // High priority
        QosAction::Allow { priority: 0 },
        0, // Any source IP
        0, // Any source netmask
        0, // Any destination IP
        0, // Any destination netmask
        17, // UDP protocol
        0, // Any source port
        5060, // SIP destination port
        0, // Any minimum packet size
        0, // Any maximum packet size
        TrafficClass::Voice,
    ).expect("Failed to create VoIP QoS rule");
    
    crate::info!("network/qos_example: Created VoIP rule with ID {}", voip_rule_id);
    
    // Create a rule for SSH traffic (high priority)
    let ssh_rule_id = create_rule(
        20, // Medium-high priority
        QosAction::Allow { priority: 1 },
        0, // Any source IP
        0, // Any source netmask
        0, // Any destination IP
        0, // Any destination netmask
        6, // TCP protocol
        0, // Any source port
        22, // SSH destination port
        0, // Any minimum packet size
        0, // Any maximum packet size
        TrafficClass::NetworkControl,
    ).expect("Failed to create SSH QoS rule");
    
    crate::info!("network/qos_example: Created SSH rule with ID {}", ssh_rule_id);
    
    // List all configured rules
    let rules = list_rules();
    crate::info!("network/qos_example: Configured {} QoS rules:", rules.len());
    
    crate::info!("network/qos_example: QoS configuration completed");
}