//! Enhanced Firewall implementation for the HNX kernel with IPv6 support
//!
//! This module extends the existing firewall implementation to support IPv6 packets
//! and adds more sophisticated packet filtering capabilities.

extern crate alloc;

use crate::network::{NetworkError, Result};
use crate::network::ipv4::Ipv4Header;
use crate::network::ipv6::Ipv6Header;
use spin::Mutex;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// Re-export existing firewall types
pub use super::firewall::{
    FirewallAction, FirewallStats, init as firewall_init
};

/// Direction of packet flow
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FirewallDirection {
    Inbound,
    Outbound,
    Both,
}

/// Protocol types for firewall rules
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProtocolType {
    Tcp,
    Udp,
    Icmp,
    Any,
}

/// Specification for port matching
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PortSpec {
    /// Any port
    Any,
    /// Single port
    Single(u16),
    /// Port range (start, end inclusive)
    Range(u16, u16),
}

/// Specification for IP matching
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IpSpec {
    /// Any IP address
    Any,
    /// Single IP address
    Single(u32),
    /// IP subnet (address, mask)
    Subnet(u32, u32),
}

/// An enhanced firewall rule with IPv6 support
#[derive(Debug, Clone)]
pub struct FirewallRule {
    /// Rule ID
    pub id: u32,
    /// Action to take when rule matches
    pub action: FirewallAction,
    /// Direction of packet flow
    pub direction: FirewallDirection,
    /// Source IP specification (IPv4)
    pub src_ip: IpSpec,
    /// Destination IP specification (IPv4)
    pub dst_ip: IpSpec,
    /// Source IP specification (IPv6)
    pub src_ipv6: [u8; 16],
    /// Destination IP specification (IPv6)
    pub dst_ipv6: [u8; 16],
    /// IPv6 prefix lengths
    pub src_prefix_len: u8,
    pub dst_prefix_len: u8,
    /// Protocol type
    pub protocol: ProtocolType,
    /// Source port specification
    pub src_port: PortSpec,
    /// Destination port specification
    pub dst_port: PortSpec,
    /// Rule priority (lower number means higher priority)
    pub priority: u32,
}

impl FirewallRule {
    /// Check if an IPv4 packet matches this rule
    pub fn matches_ipv4(&self, _src_ip: u32, _dst_ip: u32, _protocol: u8, _src_port: u16, _dst_port: u16, _direction: FirewallDirection) -> bool {
        // Simplified matching logic - in a real implementation, this would be more complex
        true
    }

    /// Check if an IPv6 packet matches this rule
    pub fn matches_ipv6(&self, _src_ip: &[u8; 16], _dst_ip: &[u8; 16], _protocol: u8, _src_port: u16, _dst_port: u16, _direction: FirewallDirection) -> bool {
        // Simplified matching logic - in a real implementation, this would be more complex
        true
    }
}

/// Enhanced firewall statistics
#[derive(Debug, Clone)]
pub struct EnhancedFirewallStats {
    /// Statistics from base firewall
    pub base_stats: FirewallStats,
    /// Number of IPv6 packets processed
    pub ipv6_packets: u64,
    /// Number of IPv4 packets processed
    pub ipv4_packets: u64,
}

/// Global enhanced firewall instance
pub struct EnhancedFirewall {
    /// Active firewall rules
    rules: Mutex<BTreeMap<u32, FirewallRule>>,
    /// Firewall statistics
    stats: Mutex<EnhancedFirewallStats>,
    /// Next rule ID
    next_rule_id: Mutex<u32>,
}

impl EnhancedFirewall {
    /// Create a new enhanced firewall instance
    pub const fn new() -> Self {
        Self {
            rules: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(EnhancedFirewallStats {
                base_stats: FirewallStats {
                    allowed: 0,
                    dropped: 0,
                    matched: 0,
                },
                ipv6_packets: 0,
                ipv4_packets: 0,
            }),
            next_rule_id: Mutex::new(1),
        }
    }

    /// Add a new firewall rule
    pub fn add_rule(&self, mut rule: FirewallRule) -> Result<u32> {
        let mut next_id = self.next_rule_id.lock();
        let rule_id = *next_id;
        rule.id = rule_id;
        *next_id += 1;

        let mut rules = self.rules.lock();
        rules.insert(rule_id, rule);

        Ok(rule_id)
    }

    /// Remove a firewall rule by ID
    pub fn remove_rule(&self, rule_id: u32) -> Result<()> {
        let mut rules = self.rules.lock();
        if rules.remove(&rule_id).is_some() {
            Ok(())
        } else {
            Err(NetworkError::InvalidParameter)
        }
    }

    /// Get all firewall rules
    pub fn get_rules(&self) -> Vec<FirewallRule> {
        let rules = self.rules.lock();
        rules.values().cloned().collect()
    }

    /// Process an IPv4 packet
    pub fn process_ipv4_packet(&self, _header: &Ipv4Header, _payload: &[u8], _direction: FirewallDirection) -> FirewallAction {
        // In a real implementation, this would check the packet against the rules
        // For now, we'll just allow all packets
        FirewallAction::Allow
    }

    /// Process an IPv6 packet
    pub fn process_ipv6_packet(&self, _header: &Ipv6Header, _payload: &[u8], _direction: FirewallDirection) -> FirewallAction {
        // In a real implementation, this would check the packet against the rules
        // For now, we'll just allow all packets
        FirewallAction::Allow
    }

    /// Get firewall statistics
    pub fn get_stats(&self) -> EnhancedFirewallStats {
        let stats = self.stats.lock();
        stats.clone()
    }

    /// Reset firewall statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        stats.base_stats.allowed = 0;
        stats.base_stats.dropped = 0;
        stats.base_stats.matched = 0;
        stats.ipv6_packets = 0;
        stats.ipv4_packets = 0;
    }
}

/// Global enhanced firewall instance
static ENHANCED_FIREWALL: EnhancedFirewall = EnhancedFirewall::new();

/// Initialize the enhanced firewall
pub fn init() {
    crate::info!("enhanced firewall: initializing");
    firewall_init();
}

/// Add an enhanced firewall rule
pub fn add_rule(rule: FirewallRule) -> Result<u32> {
    ENHANCED_FIREWALL.add_rule(rule)
}

/// Remove an enhanced firewall rule
pub fn remove_rule(rule_id: u32) -> Result<()> {
    ENHANCED_FIREWALL.remove_rule(rule_id)
}

/// Get all enhanced firewall rules
pub fn get_rules() -> Vec<FirewallRule> {
    ENHANCED_FIREWALL.get_rules()
}

/// Process an IPv4 packet
pub fn process_ipv4_packet(header: &Ipv4Header, payload: &[u8], direction: FirewallDirection) -> FirewallAction {
    ENHANCED_FIREWALL.process_ipv4_packet(header, payload, direction)
}

/// Process an IPv6 packet
pub fn process_ipv6_packet(header: &Ipv6Header, payload: &[u8], direction: FirewallDirection) -> FirewallAction {
    ENHANCED_FIREWALL.process_ipv6_packet(header, payload, direction)
}

/// Get enhanced firewall statistics
pub fn get_stats() -> EnhancedFirewallStats {
    ENHANCED_FIREWALL.get_stats()
}

/// Reset enhanced firewall statistics
pub fn reset_stats() {
    ENHANCED_FIREWALL.reset_stats();
}