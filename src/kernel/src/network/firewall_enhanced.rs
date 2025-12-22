//! Enhanced Firewall implementation for the HNX kernel with IPv6 support
//!
//! This module extends the existing firewall implementation to support IPv6 packets
//! and adds more sophisticated packet filtering capabilities.

extern crate alloc;

use crate::network::{NetworkError, Result};
use crate::network::ipv4::Ipv4Header;
use crate::network::ipv6::Ipv6Header;
use crate::network::tcp::TcpHeader;
use crate::network::udp::UdpHeader;
use spin::Mutex;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// Re-export existing firewall types
pub use super::firewall::{
    FirewallAction, FirewallDirection, ProtocolType, PortSpec, IpSpec,
    FirewallRule, FirewallStats, enable, disable, is_enabled, add_rule,
    create_rule, remove_rule, list_rules, get_stats, reset_stats,
    process_incoming_packet, process_outgoing_packet
};

/// IPv6 address specification for firewall rules
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ipv6Spec {
    /// Any IPv6 address
    Any,
    /// Single IPv6 address
    Single([u8; 16]),
    /// IPv6 subnet (address/prefix_length)
    Subnet([u8; 16], u8),
}

/// Enhanced firewall rule structure with IPv6 support
#[derive(Debug, Clone)]
pub struct FirewallRuleV6 {
    /// Rule ID (unique identifier)
    pub id: u32,
    /// Rule priority (lower values have higher priority)
    pub priority: u32,
    /// Action to take when rule matches
    pub action: FirewallAction,
    /// Direction of traffic this rule applies to
    pub direction: FirewallDirection,
    /// Source IPv4 address specification
    pub source_ip: IpSpec,
    /// Destination IPv4 address specification
    pub dest_ip: IpSpec,
    /// Source IPv6 address specification
    pub source_ipv6: Ipv6Spec,
    /// Destination IPv6 address specification
    pub dest_ipv6: Ipv6Spec,
    /// Protocol specification
    pub protocol: ProtocolType,
    /// Source port specification
    pub source_port: PortSpec,
    /// Destination port specification
    pub dest_port: PortSpec,
}

/// Enhanced firewall statistics with IPv6 counters
#[derive(Debug, Clone)]
pub struct FirewallStatsV6 {
    /// Number of IPv4 packets allowed
    pub ipv4_allowed_packets: u64,
    /// Number of IPv4 packets dropped
    pub ipv4_dropped_packets: u64,
    /// Number of IPv4 packets rejected
    pub ipv4_rejected_packets: u64,
    /// Number of IPv6 packets allowed
    pub ipv6_allowed_packets: u64,
    /// Number of IPv6 packets dropped
    pub ipv6_dropped_packets: u64,
    /// Number of IPv6 packets rejected
    pub ipv6_rejected_packets: u64,
}

impl FirewallStatsV6 {
    /// Create new empty firewall statistics
    const fn new() -> Self {
        FirewallStatsV6 {
            ipv4_allowed_packets: 0,
            ipv4_dropped_packets: 0,
            ipv4_rejected_packets: 0,
            ipv6_allowed_packets: 0,
            ipv6_dropped_packets: 0,
            ipv6_rejected_packets: 0,
        }
    }
}

/// Enhanced global firewall state with IPv6 support
struct FirewallStateV6 {
    /// Active IPv4 firewall rules
    ipv4_rules: BTreeMap<u32, super::firewall::FirewallRule>,
    /// Active IPv6 firewall rules
    ipv6_rules: BTreeMap<u32, FirewallRuleV6>,
    /// Firewall statistics
    stats: FirewallStatsV6,
    /// Next rule ID to assign
    next_rule_id: u32,
    /// Whether the firewall is enabled
    enabled: bool,
}

impl FirewallStateV6 {
    /// Create new firewall state
    const fn new() -> Self {
        FirewallStateV6 {
            ipv4_rules: BTreeMap::new(),
            ipv6_rules: BTreeMap::new(),
            stats: FirewallStatsV6::new(),
            next_rule_id: 1,
            enabled: false,
        }
    }
}

/// Enhanced global firewall instance
static FIREWALL_V6: Mutex<FirewallStateV6> = Mutex::new(FirewallStateV6::new());

/// Initialize the enhanced firewall subsystem
pub fn init_v6() {
    crate::info!("network/firewall_v6: initializing");
    
    let mut firewall = FIREWALL_V6.lock();
    *firewall = FirewallStateV6::new();
    
    crate::info!("network/firewall_v6: initialized");
}

/// Add a new IPv6 firewall rule
pub fn add_rule_v6(rule: FirewallRuleV6) -> Result<u32> {
    let mut firewall = FIREWALL_V6.lock();
    
    // Check if rule ID already exists
    if firewall.ipv6_rules.contains_key(&rule.id) {
        return Err(NetworkError::AddressInUse);
    }
    
    let rule_id = rule.id;
    firewall.ipv6_rules.insert(rule_id, rule);
    
    crate::info!("network/firewall_v6: added rule with ID {}", rule_id);
    
    Ok(rule_id)
}

/// Create and add a new IPv6 firewall rule with auto-assigned ID
pub fn create_rule_v6(
    priority: u32,
    action: FirewallAction,
    direction: FirewallDirection,
    source_ipv6: Ipv6Spec,
    dest_ipv6: Ipv6Spec,
    protocol: ProtocolType,
    source_port: PortSpec,
    dest_port: PortSpec,
) -> Result<u32> {
    let mut firewall = FIREWALL_V6.lock();
    
    let rule_id = firewall.next_rule_id;
    firewall.next_rule_id += 1;
    
    let rule = FirewallRuleV6 {
        id: rule_id,
        priority,
        action,
        direction,
        source_ip: IpSpec::Any, // Not used for IPv6 rules
        dest_ip: IpSpec::Any,   // Not used for IPv6 rules
        source_ipv6,
        dest_ipv6,
        protocol,
        source_port,
        dest_port,
    };
    
    firewall.ipv6_rules.insert(rule_id, rule);
    
    crate::info!("network/firewall_v6: created rule with ID {}", rule_id);
    
    Ok(rule_id)
}

/// Remove an IPv6 firewall rule by ID
pub fn remove_rule_v6(rule_id: u32) -> Result<()> {
    let mut firewall = FIREWALL_V6.lock();
    
    if firewall.ipv6_rules.remove(&rule_id).is_some() {
        crate::info!("network/firewall_v6: removed rule with ID {}", rule_id);
        Ok(())
    } else {
        Err(NetworkError::InvalidParameter)
    }
}

/// List all IPv6 firewall rules
pub fn list_rules_v6() -> Vec<FirewallRuleV6> {
    let firewall = FIREWALL_V6.lock();
    firewall.ipv6_rules.values().cloned().collect()
}

/// Get enhanced firewall statistics
pub fn get_stats_v6() -> FirewallStatsV6 {
    let firewall = FIREWALL_V6.lock();
    firewall.stats.clone()
}

/// Reset enhanced firewall statistics
pub fn reset_stats_v6() {
    let mut firewall = FIREWALL_V6.lock();
    firewall.stats = FirewallStatsV6::new();
}

/// Check if an IPv6 address matches an IPv6 specification
pub fn ipv6_matches(ip: [u8; 16], spec: &Ipv6Spec) -> bool {
    match spec {
        Ipv6Spec::Any => true,
        Ipv6Spec::Single(addr) => ip == *addr,
        Ipv6Spec::Subnet(network, prefix_length) => {
            // Compare only the network portion based on prefix length
            let bytes_to_compare = (*prefix_length as usize) / 8;
            let bits_to_compare = (*prefix_length as usize) % 8;
            
            // Compare full bytes
            if bytes_to_compare > 0 && ip[..bytes_to_compare] != network[..bytes_to_compare] {
                return false;
            }
            
            // Compare remaining bits if any
            if bits_to_compare > 0 && bytes_to_compare < 16 {
                let mask = !((1 << (8 - bits_to_compare)) - 1);
                if (ip[bytes_to_compare] & mask) != (network[bytes_to_compare] & mask) {
                    return false;
                }
            }
            
            true
        }
    }
}

/// Match an IPv6 packet against firewall rules
fn match_rules_v6(
    direction: FirewallDirection,
    ipv6_header: &Ipv6Header,
    transport_header: Option<&[u8]>,
    next_header: u8,
) -> Option<FirewallAction> {
    let firewall = FIREWALL_V6.lock();
    
    // If firewall is disabled, allow all packets
    if !firewall.enabled {
        return Some(FirewallAction::Allow);
    }
    
    // Extract port information if available
    let (src_port, dst_port) = if let Some(header_data) = transport_header {
        match next_header {
            crate::network::ipv6::next_header::TCP => {
                if header_data.len() >= TcpHeader::MIN_SIZE {
                    match TcpHeader::from_bytes(header_data) {
                        Ok(tcp_header) => (u16::from_be(tcp_header.source_port), u16::from_be(tcp_header.destination_port)),
                        Err(_) => (0, 0), // Use default values instead of early return
                    }
                } else {
                    (0, 0)
                }
            }
            crate::network::ipv6::next_header::UDP => {
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
    let mut matching_rules: Vec<&FirewallRuleV6> = Vec::new();
    
    // Process rules efficiently to avoid performance issues
    for rule in firewall.ipv6_rules.values() {
        // Check direction
        let direction_match = match rule.direction {
            FirewallDirection::Incoming => direction == FirewallDirection::Incoming,
            FirewallDirection::Outgoing => direction == FirewallDirection::Outgoing,
            FirewallDirection::Both => true,
        };
        
        if !direction_match {
            continue;
        }
        
        // Check source IPv6
        if !ipv6_matches(ipv6_header.source_addr, &rule.source_ipv6) {
            continue;
        }
        
        // Check destination IPv6
        if !ipv6_matches(ipv6_header.destination_addr, &rule.dest_ipv6) {
            continue;
        }
        
        // Check protocol
        let protocol_matches = match rule.protocol {
            ProtocolType::Any => true,
            ProtocolType::Tcp => next_header == crate::network::ipv6::next_header::TCP,
            ProtocolType::Udp => next_header == crate::network::ipv6::next_header::UDP,
            ProtocolType::Icmp => next_header == crate::network::ipv6::next_header::ICMPV6,
        };
        
        if !protocol_matches {
            continue;
        }
        
        // Check source port
        if !super::firewall::port_matches(src_port, &rule.source_port) {
            continue;
        }
        
        // Check destination port
        if !super::firewall::port_matches(dst_port, &rule.dest_port) {
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

/// Process an incoming IPv6 packet through the firewall
pub fn process_incoming_packet_v6(
    ipv6_header: &Ipv6Header,
    transport_header: Option<&[u8]>,
) -> Result<FirewallAction> {
    let action = match_rules_v6(
        FirewallDirection::Incoming,
        ipv6_header,
        transport_header,
        ipv6_header.next_header,
    ).unwrap_or(FirewallAction::Allow); // Default to allow if no rules match
    
    // Update statistics
    {
        let mut firewall = FIREWALL_V6.lock();
        match action {
            FirewallAction::Allow => firewall.stats.ipv6_allowed_packets += 1,
            FirewallAction::Drop => firewall.stats.ipv6_dropped_packets += 1,
            FirewallAction::Reject => firewall.stats.ipv6_rejected_packets += 1,
        }
    }
    
    Ok(action)
}

/// Process an outgoing IPv6 packet through the firewall
pub fn process_outgoing_packet_v6(
    ipv6_header: &Ipv6Header,
    transport_header: Option<&[u8]>,
) -> Result<FirewallAction> {
    let action = match_rules_v6(
        FirewallDirection::Outgoing,
        ipv6_header,
        transport_header,
        ipv6_header.next_header,
    ).unwrap_or(FirewallAction::Allow); // Default to allow if no rules match
    
    // Update statistics
    {
        let mut firewall = FIREWALL_V6.lock();
        match action {
            FirewallAction::Allow => firewall.stats.ipv6_allowed_packets += 1,
            FirewallAction::Drop => firewall.stats.ipv6_dropped_packets += 1,
            FirewallAction::Reject => firewall.stats.ipv6_rejected_packets += 1,
        }
    }
    
    Ok(action)
}

/// Enhanced packet filtering with connection tracking
pub mod connection_tracking {
    extern crate alloc;
    
    use spin::Mutex;
    use alloc::collections::BTreeMap;
    use crate::network::firewall::FirewallAction;
    
    /// Connection state
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum ConnectionState {
        /// New connection
        New,
        /// Established connection
        Established,
        /// Related connection (e.g., FTP data connection)
        Related,
        /// Invalid packet
        Invalid,
    }
    
    /// Connection tuple for tracking
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct ConnectionTuple {
        pub src_ip: [u8; 16],  // Using IPv6 format to accommodate both IPv4 and IPv6
        pub dst_ip: [u8; 16],
        pub src_port: u16,
        pub dst_port: u16,
        pub protocol: u8,
    }
    
    /// Tracked connection information
    #[derive(Debug, Clone)]
    pub struct TrackedConnection {
        pub state: ConnectionState,
        pub last_seen: u64, // Timestamp
        pub bytes_in: u64,
        pub bytes_out: u64,
    }
    
    /// Global connection tracking table
    static CONNECTION_TRACKING_TABLE: Mutex<BTreeMap<ConnectionTuple, TrackedConnection>> = 
        Mutex::new(BTreeMap::new());
    
    /// Initialize connection tracking
    pub fn init() {
        crate::info!("network/firewall/connection_tracking: initializing");
    }
    
    /// Add or update a tracked connection
    pub fn update_connection(tuple: ConnectionTuple, state: ConnectionState) {
        let mut table = CONNECTION_TRACKING_TABLE.lock();
        let timestamp = 0; // In a real implementation, this would be the current time
        
        table.insert(tuple, TrackedConnection {
            state,
            last_seen: timestamp,
            bytes_in: 0,
            bytes_out: 0,
        });
    }
    
    /// Check if a connection is tracked and return its state
    pub fn check_connection(tuple: &ConnectionTuple) -> Option<ConnectionState> {
        let table = CONNECTION_TRACKING_TABLE.lock();
        table.get(tuple).map(|conn| conn.state)
    }
    
    /// Remove a tracked connection
    pub fn remove_connection(tuple: &ConnectionTuple) {
        let mut table = CONNECTION_TRACKING_TABLE.lock();
        table.remove(tuple);
    }
    
    /// Clean up stale connections (would be called periodically)
    pub fn cleanup_stale_connections() {
        let mut table = CONNECTION_TRACKING_TABLE.lock();
        let timestamp = 0; // In a real implementation, this would be the current time
        let timeout = 300; // 5 minutes timeout
        
        // Remove connections that haven't been seen for more than timeout seconds
        table.retain(|_, conn| timestamp - conn.last_seen < timeout);
    }
}

/// Rate limiting functionality
pub mod rate_limiting {
    extern crate alloc;
    
    use spin::Mutex;
    use alloc::collections::BTreeMap;
    
    /// Rate limit rule
    #[derive(Debug, Clone)]
    pub struct RateLimitRule {
        pub id: u32,
        pub max_packets_per_second: u32,
        pub max_bytes_per_second: u64,
        pub burst_size: u32,
    }
    
    /// Rate limit state for a specific flow
    #[derive(Debug, Clone)]
    pub struct RateLimitState {
        pub packets_this_second: u32,
        pub bytes_this_second: u64,
        pub last_reset: u64, // Timestamp
        pub burst_tokens: u32,
    }
    
    /// Global rate limiting rules
    static RATE_LIMIT_RULES: Mutex<BTreeMap<u32, RateLimitRule>> = 
        Mutex::new(BTreeMap::new());
    
    /// Global rate limiting state
    static RATE_LIMIT_STATE: Mutex<BTreeMap<u32, RateLimitState>> = 
        Mutex::new(BTreeMap::new());
    
    /// Initialize rate limiting
    pub fn init() {
        crate::info!("network/firewall/rate_limiting: initializing");
    }
    
    /// Add a rate limit rule
    pub fn add_rate_limit_rule(rule: RateLimitRule) {
        let mut rules = RATE_LIMIT_RULES.lock();
        rules.insert(rule.id, rule);
    }
    
    /// Remove a rate limit rule
    pub fn remove_rate_limit_rule(id: u32) {
        let mut rules = RATE_LIMIT_RULES.lock();
        rules.remove(&id);
    }
    
    /// Check if a packet should be rate limited
    pub fn check_rate_limit(rule_id: u32, packet_size: usize) -> bool {
        let rules = RATE_LIMIT_RULES.lock();
        let mut states = RATE_LIMIT_STATE.lock();
        
        // Get the rule
        let rule = match rules.get(&rule_id) {
            Some(r) => r,
            None => return false, // No rule, don't rate limit
        };
        
        let timestamp = 0; // In a real implementation, this would be the current time
        let mut state = states.entry(rule_id).or_insert(RateLimitState {
            packets_this_second: 0,
            bytes_this_second: 0,
            last_reset: timestamp,
            burst_tokens: rule.burst_size,
        });
        
        // Reset counters if a second has passed
        if timestamp - state.last_reset >= 1 {
            state.packets_this_second = 0;
            state.bytes_this_second = 0;
            state.last_reset = timestamp;
            state.burst_tokens = rule.burst_size;
        }
        
        // Check if we have burst tokens
        if state.burst_tokens > 0 {
            state.burst_tokens -= 1;
            return false; // Allow packet
        }
        
        // Check packet rate limit
        if state.packets_this_second >= rule.max_packets_per_second {
            return true; // Rate limit exceeded
        }
        
        // Check byte rate limit
        if state.bytes_this_second + (packet_size as u64) > rule.max_bytes_per_second {
            return true; // Rate limit exceeded
        }
        
        // Update counters
        state.packets_this_second += 1;
        state.bytes_this_second += packet_size as u64;
        
        false // Don't rate limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ipv6_matching() {
        // Test exact match
        let addr1: [u8; 16] = [0x20, 0x01, 0x0d, 0xb8, 0x00, 0x00, 0x00, 0x00, 
                              0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
        let addr2: [u8; 16] = [0x20, 0x01, 0x0d, 0xb8, 0x00, 0x00, 0x00, 0x00, 
                              0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];
        let spec = Ipv6Spec::Single(addr1);
        assert!(ipv6_matches(addr2, &spec));
        
        // Test subnet match
        let network: [u8; 16] = [0x20, 0x01, 0x0d, 0xb8, 0x00, 0x00, 0x00, 0x00, 
                                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let addr_in_subnet: [u8; 16] = [0x20, 0x01, 0x0d, 0xb8, 0x00, 0x00, 0x00, 0x00, 
                                       0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05];
        let spec = Ipv6Spec::Subnet(network, 64);
        assert!(ipv6_matches(addr_in_subnet, &spec));
        
        // Test mismatch
        let addr3: [u8; 16] = [0x20, 0x01, 0x0d, 0xb8, 0x00, 0x00, 0x00, 0x00, 
                              0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02];
        assert!(!ipv6_matches(addr3, &spec));
    }
    
    #[test]
    fn test_firewall_v6_rule_creation() {
        // Test creating an IPv6 firewall rule
        let source_ipv6 = Ipv6Spec::Any;
        let dest_ipv6 = Ipv6Spec::Single([0; 16]);
        let protocol = ProtocolType::Tcp;
        let source_port = PortSpec::Any;
        let dest_port = PortSpec::Single(80);
        
        let rule_id = create_rule_v6(
            100,
            FirewallAction::Allow,
            FirewallDirection::Incoming,
            source_ipv6,
            dest_ipv6,
            protocol,
            source_port,
            dest_port,
        );
        
        assert!(rule_id.is_ok());
        
        // Verify the rule was added
        let rules = list_rules_v6();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, rule_id.unwrap());
    }
}