//! Firewall implementation for the HNX kernel
//!
//! This module provides a simple packet filtering firewall that can be used
//! to control network traffic based on rules defined by the administrator.

extern crate alloc;

use crate::network::{NetworkError, Result};
use crate::network::ipv4::Ipv4Header;
use crate::network::tcp::TcpHeader;
use crate::network::udp::UdpHeader;
use spin::Mutex;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Firewall rule action
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FirewallAction {
    /// Allow the packet to pass through
    Allow,
    /// Drop the packet silently
    Drop,
    /// Reject the packet with an ICMP response
    Reject,
}

/// Firewall rule direction
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FirewallDirection {
    /// Incoming packets (received from network)
    Incoming,
    /// Outgoing packets (sent to network)
    Outgoing,
    /// Both directions
    Both,
}

/// Protocol type for firewall rules
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProtocolType {
    /// Any protocol
    Any,
    /// TCP protocol
    Tcp,
    /// UDP protocol
    Udp,
    /// ICMP protocol
    Icmp,
}

/// Port specification for firewall rules
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PortSpec {
    /// Any port
    Any,
    /// Single port
    Single(u16),
    /// Port range (inclusive)
    Range(u16, u16),
}

/// IP address specification for firewall rules
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IpSpec {
    /// Any IP address
    Any,
    /// Single IP address
    Single(u32),
    /// IP subnet (address/netmask)
    Subnet(u32, u32),
}

/// Firewall rule structure
#[derive(Debug, Clone)]
pub struct FirewallRule {
    /// Rule ID (unique identifier)
    pub id: u32,
    /// Rule priority (lower values have higher priority)
    pub priority: u32,
    /// Action to take when rule matches
    pub action: FirewallAction,
    /// Direction of traffic this rule applies to
    pub direction: FirewallDirection,
    /// Source IP address specification
    pub source_ip: IpSpec,
    /// Destination IP address specification
    pub dest_ip: IpSpec,
    /// Protocol specification
    pub protocol: ProtocolType,
    /// Source port specification
    pub source_port: PortSpec,
    /// Destination port specification
    pub dest_port: PortSpec,
}

/// Firewall statistics
#[derive(Debug, Clone)]
pub struct FirewallStats {
    /// Number of packets allowed
    pub allowed_packets: u64,
    /// Number of packets dropped
    pub dropped_packets: u64,
    /// Number of packets rejected
    pub rejected_packets: u64,
}

impl FirewallStats {
    /// Create new empty firewall statistics
    const fn new() -> Self {
        FirewallStats {
            allowed_packets: 0,
            dropped_packets: 0,
            rejected_packets: 0,
        }
    }
}

/// Global firewall state
struct FirewallState {
    /// Active firewall rules
    rules: BTreeMap<u32, FirewallRule>,
    /// Firewall statistics
    stats: FirewallStats,
    /// Next rule ID to assign
    next_rule_id: u32,
    /// Whether the firewall is enabled
    enabled: bool,
}

impl FirewallState {
    /// Create new firewall state
    const fn new() -> Self {
        FirewallState {
            rules: BTreeMap::new(),
            stats: FirewallStats::new(),
            next_rule_id: 1,
            enabled: false,
        }
    }
}

/// Global firewall instance
static FIREWALL: Mutex<FirewallState> = Mutex::new(FirewallState::new());

/// Initialize the firewall subsystem
pub fn init() {
    crate::info!("network/firewall: initializing");
    
    let mut firewall = FIREWALL.lock();
    *firewall = FirewallState::new();
    
    crate::info!("network/firewall: initialized");
}

/// Enable the firewall
pub fn enable() {
    let mut firewall = FIREWALL.lock();
    firewall.enabled = true;
    crate::info!("network/firewall: enabled");
}

/// Disable the firewall
pub fn disable() {
    let mut firewall = FIREWALL.lock();
    firewall.enabled = false;
    crate::info!("network/firewall: disabled");
}

/// Check if the firewall is enabled
pub fn is_enabled() -> bool {
    let firewall = FIREWALL.lock();
    firewall.enabled
}

/// Add a new firewall rule
pub fn add_rule(rule: FirewallRule) -> Result<u32> {
    let mut firewall = FIREWALL.lock();
    
    // Check if rule ID already exists
    if firewall.rules.contains_key(&rule.id) {
        return Err(NetworkError::AddressInUse);
    }
    
    let rule_id = rule.id;
    firewall.rules.insert(rule_id, rule);
    
    crate::info!("network/firewall: added rule with ID {}", rule_id);
    
    Ok(rule_id)
}

/// Create and add a new firewall rule with auto-assigned ID
pub fn create_rule(
    priority: u32,
    action: FirewallAction,
    direction: FirewallDirection,
    source_ip: IpSpec,
    dest_ip: IpSpec,
    protocol: ProtocolType,
    source_port: PortSpec,
    dest_port: PortSpec,
) -> Result<u32> {
    let mut firewall = FIREWALL.lock();
    
    let rule_id = firewall.next_rule_id;
    firewall.next_rule_id += 1;
    
    let rule = FirewallRule {
        id: rule_id,
        priority,
        action,
        direction,
        source_ip,
        dest_ip,
        protocol,
        source_port,
        dest_port,
    };
    
    firewall.rules.insert(rule_id, rule);
    
    crate::info!("network/firewall: created rule with ID {}", rule_id);
    
    Ok(rule_id)
}

/// Remove a firewall rule by ID
pub fn remove_rule(rule_id: u32) -> Result<()> {
    let mut firewall = FIREWALL.lock();
    
    if firewall.rules.remove(&rule_id).is_some() {
        crate::info!("network/firewall: removed rule with ID {}", rule_id);
        Ok(())
    } else {
        Err(NetworkError::InvalidParameter)
    }
}

/// List all firewall rules
pub fn list_rules() -> Vec<FirewallRule> {
    let firewall = FIREWALL.lock();
    firewall.rules.values().cloned().collect()
}

/// Get firewall statistics
pub fn get_stats() -> FirewallStats {
    let firewall = FIREWALL.lock();
    firewall.stats.clone()
}

/// Reset firewall statistics
pub fn reset_stats() {
    let mut firewall = FIREWALL.lock();
    firewall.stats = FirewallStats::new();
}

/// Check if an IP address matches an IP specification
pub fn ip_matches(ip: u32, spec: &IpSpec) -> bool {
    match spec {
        IpSpec::Any => true,
        IpSpec::Single(addr) => ip == *addr,
        IpSpec::Subnet(network, netmask) => (ip & netmask) == (network & netmask),
    }
}

/// Check if a port matches a port specification
pub fn port_matches(port: u16, spec: &PortSpec) -> bool {
    match spec {
        PortSpec::Any => true,
        PortSpec::Single(p) => port == *p,
        PortSpec::Range(start, end) => port >= *start && port <= *end,
    }
}

/// Check if a protocol matches a protocol specification
fn protocol_matches(protocol: u8, spec: &ProtocolType) -> bool {
    match spec {
        ProtocolType::Any => true,
        ProtocolType::Tcp => protocol == crate::network::ipv4::protocol::TCP,
        ProtocolType::Udp => protocol == crate::network::ipv4::protocol::UDP,
        ProtocolType::Icmp => protocol == crate::network::ipv4::protocol::ICMP,
    }
}

/// Match a packet against firewall rules
fn match_rules(
    direction: FirewallDirection,
    ip_header: &Ipv4Header,
    transport_header: Option<&[u8]>,
    protocol: u8,
) -> Option<FirewallAction> {
    let firewall = FIREWALL.lock();
    
    // If firewall is disabled, allow all packets
    if !firewall.enabled {
        return Some(FirewallAction::Allow);
    }
    
    // Convert IP addresses from network byte order
    let src_ip = u32::from_be(ip_header.source_ip);
    let dst_ip = u32::from_be(ip_header.destination_ip);
    
    // Extract port information if available
    let (src_port, dst_port) = if let Some(header_data) = transport_header {
        match protocol {
            crate::network::ipv4::protocol::TCP => {
                if header_data.len() >= TcpHeader::MIN_SIZE {
                    match TcpHeader::from_bytes(header_data) {
                        Ok(tcp_header) => (u16::from_be(tcp_header.source_port), u16::from_be(tcp_header.destination_port)),
                        Err(_) => (0, 0), // Use default values instead of early return
                    }
                } else {
                    (0, 0)
                }
            }
            crate::network::ipv4::protocol::UDP => {
                if header_data.len() >= UdpHeader::SIZE {
                    match UdpHeader::from_bytes(header_data) {
                        Ok(udp_header) => (u16::from_be(udp_header.source_port), u16::from_be(udp_header.destination_port)),
                        Err(_) => (0, 0), // Use default values instead of early return
                    }
                } else {
                    (0, 0)
                }
            }
            _ => (0, 0),
        }
    } else {
        (0, 0)
    };
    
    // Collect matching rules and sort by priority
    let mut matching_rules: Vec<&FirewallRule> = Vec::new();
    
    // Process rules efficiently to avoid performance issues
    for rule in firewall.rules.values() {
        // Check direction
        let direction_match = match rule.direction {
            FirewallDirection::Incoming => direction == FirewallDirection::Incoming,
            FirewallDirection::Outgoing => direction == FirewallDirection::Outgoing,
            FirewallDirection::Both => true,
        };
        
        if !direction_match {
            continue;
        }
        
        // Check source IP
        if !ip_matches(src_ip, &rule.source_ip) {
            continue;
        }
        
        // Check destination IP
        if !ip_matches(dst_ip, &rule.dest_ip) {
            continue;
        }
        
        // Check protocol
        if !protocol_matches(protocol, &rule.protocol) {
            continue;
        }
        
        // Check source port
        if !port_matches(src_port, &rule.source_port) {
            continue;
        }
        
        // Check destination port
        if !port_matches(dst_port, &rule.dest_port) {
            continue;
        }
        
        // If we reach here, the rule matches
        matching_rules.push(rule);
    }
    
    // Sort by priority (lowest first)
    matching_rules.sort_by_key(|rule| rule.priority);
    
    // Return action of highest priority rule, or None if no rules match
    matching_rules.first().map(|rule| rule.action)
}

/// Process an incoming packet through the firewall
pub fn process_incoming_packet(
    ip_header: &Ipv4Header,
    transport_header: Option<&[u8]>,
) -> Result<FirewallAction> {
    let action = match_rules(
        FirewallDirection::Incoming,
        ip_header,
        transport_header,
        ip_header.protocol,
    ).unwrap_or(FirewallAction::Allow); // Default to allow if no rules match
    
    // Update statistics
    {
        let mut firewall = FIREWALL.lock();
        match action {
            FirewallAction::Allow => firewall.stats.allowed_packets += 1,
            FirewallAction::Drop => firewall.stats.dropped_packets += 1,
            FirewallAction::Reject => firewall.stats.rejected_packets += 1,
        }
    }
    
    Ok(action)
}

/// Process an outgoing packet through the firewall
pub fn process_outgoing_packet(
    ip_header: &Ipv4Header,
    transport_header: Option<&[u8]>,
) -> Result<FirewallAction> {
    let action = match_rules(
        FirewallDirection::Outgoing,
        ip_header,
        transport_header,
        ip_header.protocol,
    ).unwrap_or(FirewallAction::Allow); // Default to allow if no rules match
    
    // Update statistics
    {
        let mut firewall = FIREWALL.lock();
        match action {
            FirewallAction::Allow => firewall.stats.allowed_packets += 1,
            FirewallAction::Drop => firewall.stats.dropped_packets += 1,
            FirewallAction::Reject => firewall.stats.rejected_packets += 1,
        }
    }
    
    Ok(action)
}