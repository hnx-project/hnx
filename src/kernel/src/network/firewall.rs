//! Firewall implementation for the HNX kernel
//!
//! This module provides a simple packet filtering firewall that can be used
//! to control network traffic based on rules defined by the administrator.

extern crate alloc;

use crate::network::{NetworkError, Result};
use crate::network::ipv4::Ipv4Header;
use spin::Mutex;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Firewall rule action
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FirewallAction {
    /// Allow the packet to pass through
    Allow,
    /// Drop the packet
    Drop,
    /// Reject the packet (send back an error)
    Reject,
}

/// Protocol types for firewall rules
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProtocolType {
    Tcp,
    Udp,
    Icmp,
    Any,
}

/// Direction of packet flow
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Inbound,
    Outbound,
    Both,
}

/// A firewall rule
#[derive(Debug, Clone)]
pub struct FirewallRule {
    /// Rule ID
    pub id: u32,
    /// Action to take when rule matches
    pub action: FirewallAction,
    /// Direction of packet flow
    pub direction: Direction,
    /// Source IP address (0.0.0.0 means any)
    pub src_ip: u32,
    /// Source IP mask
    pub src_mask: u32,
    /// Destination IP address (0.0.0.0 means any)
    pub dst_ip: u32,
    /// Destination IP mask
    pub dst_mask: u32,
    /// Protocol type
    pub protocol: ProtocolType,
    /// Source port (0 means any)
    pub src_port: u16,
    /// Destination port (0 means any)
    pub dst_port: u16,
    /// Rule priority (lower number means higher priority)
    pub priority: u32,
}

impl FirewallRule {
    /// Check if a packet matches this rule
    pub fn matches(&self, _src_ip: u32, _dst_ip: u32, _protocol: u8, _src_port: u16, _dst_port: u16, _direction: Direction) -> bool {
        // Simplified matching logic - in a real implementation, this would be more complex
        true
    }
}

/// Firewall statistics
#[derive(Debug, Clone)]
pub struct FirewallStats {
    /// Number of packets allowed
    pub allowed: u64,
    /// Number of packets dropped
    pub dropped: u64,
    /// Number of packets that matched rules
    pub matched: u64,
}

/// Global firewall instance
pub struct Firewall {
    /// Active firewall rules
    rules: Mutex<BTreeMap<u32, FirewallRule>>,
    /// Firewall statistics
    stats: Mutex<FirewallStats>,
    /// Next rule ID
    next_rule_id: Mutex<u32>,
}

impl Firewall {
    /// Create a new firewall instance
    pub const fn new() -> Self {
        Self {
            rules: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(FirewallStats {
                allowed: 0,
                dropped: 0,
                matched: 0,
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
    pub fn process_ipv4_packet(&self, _header: &Ipv4Header, _payload: &[u8], _direction: Direction) -> FirewallAction {
        // In a real implementation, this would check the packet against the rules
        // For now, we'll just allow all packets
        FirewallAction::Allow
    }

    /// Get firewall statistics
    pub fn get_stats(&self) -> FirewallStats {
        let stats = self.stats.lock();
        stats.clone()
    }

    /// Reset firewall statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        stats.allowed = 0;
        stats.dropped = 0;
        stats.matched = 0;
    }
}

/// Global firewall instance
static FIREWALL: Firewall = Firewall::new();

/// Initialize the firewall
pub fn init() {
    crate::info!("firewall: initializing");
}

/// Add a firewall rule
pub fn add_rule(rule: FirewallRule) -> Result<u32> {
    FIREWALL.add_rule(rule)
}

/// Remove a firewall rule
pub fn remove_rule(rule_id: u32) -> Result<()> {
    FIREWALL.remove_rule(rule_id)
}

/// Get all firewall rules
pub fn get_rules() -> Vec<FirewallRule> {
    FIREWALL.get_rules()
}

/// Process an IPv4 packet
pub fn process_ipv4_packet(header: &Ipv4Header, payload: &[u8], direction: Direction) -> FirewallAction {
    FIREWALL.process_ipv4_packet(header, payload, direction)
}

/// Get firewall statistics
pub fn get_stats() -> FirewallStats {
    FIREWALL.get_stats()
}

/// Reset firewall statistics
pub fn reset_stats() {
    FIREWALL.reset_stats();
}