//! Quality of Service (QoS) implementation for the HNX kernel
//!
//! This module provides traffic classification, marking, scheduling,
//! and policing capabilities to ensure network quality of service.

extern crate alloc;

use crate::network::{NetworkError, Result};
use crate::network::ipv4::Ipv4Header;
use crate::network::tcp::TcpHeader;
use crate::network::udp::UdpHeader;
use spin::Mutex;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;

/// QoS policy action
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QosAction {
    /// Allow traffic with specified priority
    Allow { priority: u8 },
    /// Drop traffic
    Drop,
    /// Shape traffic to specified rate (bytes per second)
    Shape { rate_bps: u64 },
    /// Police traffic to specified rate (bytes per second)
    Police { rate_bps: u64 },
}

/// Traffic class
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrafficClass {
    /// Best effort traffic (default)
    BestEffort,
    /// Background traffic (low priority)
    Background,
    /// Excellent effort traffic
    ExcellentEffort,
    /// Controlled load traffic
    ControlledLoad,
    /// Video traffic
    Video,
    /// Voice traffic
    Voice,
    /// Network control traffic
    NetworkControl,
    /// Custom traffic class
    Custom(u8),
}

/// DSCP (Differentiated Services Code Point) values
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DscpValue {
    /// Best effort
    BestEffort,
    /// Background
    Background,
    /// Excellent effort
    ExcellentEffort,
    /// Controlled load
    ControlledLoad,
    /// Video
    Video,
    /// Voice
    Voice,
    /// Network control
    NetworkControl,
    /// Custom value
    Custom(u8),
}

/// QoS rule for traffic classification
#[derive(Debug, Clone)]
pub struct QosRule {
    /// Rule ID (unique identifier)
    pub id: u32,
    /// Rule priority (lower values have higher priority)
    pub priority: u32,
    /// Action to take when rule matches
    pub action: QosAction,
    /// Source IP address specification
    pub source_ip: u32,
    /// Source IP netmask
    pub source_netmask: u32,
    /// Destination IP address specification
    pub dest_ip: u32,
    /// Destination IP netmask
    pub dest_netmask: u32,
    /// Protocol specification
    pub protocol: u8,
    /// Source port (0 for any)
    pub source_port: u16,
    /// Destination port (0 for any)
    pub dest_port: u16,
    /// Minimum packet size (0 for any)
    pub min_packet_size: u16,
    /// Maximum packet size (0 for any)
    pub max_packet_size: u16,
    /// Traffic class to assign
    pub traffic_class: TrafficClass,
}

/// QoS statistics
#[derive(Debug, Clone)]
pub struct QosStats {
    /// Number of packets classified
    pub classified_packets: u64,
    /// Number of bytes classified
    pub classified_bytes: u64,
    /// Number of packets dropped
    pub dropped_packets: u64,
    /// Number of bytes dropped
    pub dropped_bytes: u64,
    /// Number of packets shaped
    pub shaped_packets: u64,
    /// Number of bytes shaped
    pub shaped_bytes: u64,
    /// Number of packets policed
    pub policed_packets: u64,
    /// Number of bytes policed
    pub policed_bytes: u64,
}

impl QosStats {
    /// Create new empty QoS statistics
    const fn new() -> Self {
        QosStats {
            classified_packets: 0,
            classified_bytes: 0,
            dropped_packets: 0,
            dropped_bytes: 0,
            shaped_packets: 0,
            shaped_bytes: 0,
            policed_packets: 0,
            policed_bytes: 0,
        }
    }
}

/// Traffic shaper for rate limiting
#[derive(Debug, Clone)]
pub struct TrafficShaper {
    /// Current token bucket level
    current_tokens: u64,
    /// Maximum token bucket size
    max_tokens: u64,
    /// Token refill rate (tokens per millisecond)
    refill_rate_ms: u64,
    /// Last update timestamp
    last_update: u64, // milliseconds
}

impl TrafficShaper {
    /// Create a new traffic shaper
    pub fn new(rate_bps: u64) -> Self {
        // Convert rate to tokens per millisecond
        let refill_rate_ms = rate_bps / 8000; // bytes per millisecond
        
        TrafficShaper {
            current_tokens: 0,
            max_tokens: rate_bps, // Allow burst up to 1 second worth of data
            refill_rate_ms,
            last_update: 0,
        }
    }
    
    /// Check if we can consume the specified number of bytes
    pub fn can_consume(&mut self, bytes: u64, current_time_ms: u64) -> bool {
        // Update token bucket
        self.update_tokens(current_time_ms);
        
        if self.current_tokens >= bytes {
            self.current_tokens -= bytes;
            true
        } else {
            false
        }
    }
    
    /// Update token bucket based on elapsed time
    fn update_tokens(&mut self, current_time_ms: u64) {
        if current_time_ms > self.last_update {
            let elapsed_ms = current_time_ms - self.last_update;
            // Prevent overflow by checking bounds
            let new_tokens = elapsed_ms.saturating_mul(self.refill_rate_ms);
            self.current_tokens = self.current_tokens.saturating_add(new_tokens);
            // Ensure we don't exceed max_tokens
            if self.current_tokens > self.max_tokens {
                self.current_tokens = self.max_tokens;
            }
            self.last_update = current_time_ms;
        } else if current_time_ms < self.last_update {
            // Handle potential timer wraparound
            self.last_update = current_time_ms;
        }
    }
}

/// Global QoS state
struct QosState {
    /// Active QoS rules
    rules: BTreeMap<u32, QosRule>,
    /// QoS statistics
    stats: QosStats,
    /// Next rule ID to assign
    next_rule_id: u32,
    /// Whether QoS is enabled
    enabled: bool,
    /// Traffic shapers by rule ID
    shapers: BTreeMap<u32, TrafficShaper>,
}

impl QosState {
    /// Create new QoS state
    const fn new() -> Self {
        QosState {
            rules: BTreeMap::new(),
            stats: QosStats::new(),
            next_rule_id: 1,
            enabled: false,
            shapers: BTreeMap::new(),
        }
    }
}

/// Global QoS instance
static QOS: Mutex<QosState> = Mutex::new(QosState::new());

/// Initialize the QoS subsystem
pub fn init() {
    crate::info!("network/qos: initializing");
    
    let mut qos = QOS.lock();
    *qos = QosState::new();
    
    crate::info!("network/qos: initialized");
}

/// Enable the QoS system
pub fn enable() {
    let mut qos = QOS.lock();
    qos.enabled = true;
    crate::info!("network/qos: enabled");
}

/// Disable the QoS system
pub fn disable() {
    let mut qos = QOS.lock();
    qos.enabled = false;
    crate::info!("network/qos: disabled");
}

/// Check if the QoS system is enabled
pub fn is_enabled() -> bool {
    let qos = QOS.lock();
    qos.enabled
}

/// Add a new QoS rule
pub fn add_rule(rule: QosRule) -> Result<u32> {
    let mut qos = QOS.lock();
    
    // Check if rule ID already exists
    if qos.rules.contains_key(&rule.id) {
        return Err(NetworkError::AddressInUse);
    }
    
    // If the rule has shaping or policing, create a traffic shaper
    let rule_id = rule.id;
    match rule.action {
        QosAction::Shape { rate_bps } | QosAction::Police { rate_bps } => {
            let shaper = TrafficShaper::new(rate_bps);
            qos.shapers.insert(rule_id, shaper);
        }
        _ => {}
    }
    
    qos.rules.insert(rule_id, rule);
    
    crate::info!("network/qos: added rule with ID {}", rule_id);
    
    Ok(rule_id)
}

/// Create and add a new QoS rule with auto-assigned ID
pub fn create_rule(
    priority: u32,
    action: QosAction,
    source_ip: u32,
    source_netmask: u32,
    dest_ip: u32,
    dest_netmask: u32,
    protocol: u8,
    source_port: u16,
    dest_port: u16,
    min_packet_size: u16,
    max_packet_size: u16,
    traffic_class: TrafficClass,
) -> Result<u32> {
    let mut qos = QOS.lock();
    
    let rule_id = qos.next_rule_id;
    qos.next_rule_id += 1;
    
    let rule = QosRule {
        id: rule_id,
        priority,
        action,
        source_ip,
        source_netmask,
        dest_ip,
        dest_netmask,
        protocol,
        source_port,
        dest_port,
        min_packet_size,
        max_packet_size,
        traffic_class,
    };
    
    // If the rule has shaping or policing, create a traffic shaper
    match action {
        QosAction::Shape { rate_bps } | QosAction::Police { rate_bps } => {
            let shaper = TrafficShaper::new(rate_bps);
            qos.shapers.insert(rule_id, shaper);
        }
        _ => {}
    }
    
    qos.rules.insert(rule_id, rule);
    
    crate::info!("network/qos: created rule with ID {}", rule_id);
    
    Ok(rule_id)
}

/// Remove a QoS rule by ID
pub fn remove_rule(rule_id: u32) -> Result<()> {
    let mut qos = QOS.lock();
    
    if qos.rules.remove(&rule_id).is_some() {
        // Also remove associated shaper if it exists
        qos.shapers.remove(&rule_id);
        crate::info!("network/qos: removed rule with ID {}", rule_id);
        Ok(())
    } else {
        Err(NetworkError::InvalidParameter)
    }
}

/// List all QoS rules
pub fn list_rules() -> Vec<QosRule> {
    let qos = QOS.lock();
    qos.rules.values().cloned().collect()
}

/// Get QoS statistics
pub fn get_stats() -> QosStats {
    let qos = QOS.lock();
    qos.stats.clone()
}

/// Reset QoS statistics
pub fn reset_stats() {
    let mut qos = QOS.lock();
    qos.stats = QosStats::new();
}

/// Check if an IP address matches an IP specification with netmask
fn ip_matches(ip: u32, target_ip: u32, netmask: u32) -> bool {
    (ip & netmask) == (target_ip & netmask)
}

/// Check if a port is in range
fn port_matches(port: u16, target_port: u16) -> bool {
    target_port == 0 || port == target_port
}

/// Check if packet size is in range
fn size_matches(size: u16, min_size: u16, max_size: u16) -> bool {
    (min_size == 0 || size >= min_size) && (max_size == 0 || size <= max_size)
}

/// Match a packet against QoS rules
fn match_rules(
    ip_header: &Ipv4Header,
    transport_header: Option<&[u8]>,
    packet_size: usize,
) -> Option<(QosAction, TrafficClass, u32)> {
    let qos = QOS.lock();
    
    // If QoS is disabled, don't classify packets
    if !qos.enabled {
        return None;
    }
    
    // Convert IP addresses from network byte order
    let src_ip = u32::from_be(ip_header.source_ip);
    let dst_ip = u32::from_be(ip_header.destination_ip);
    
    // Extract port information if available
    let (src_port, dst_port) = if let Some(header_data) = transport_header {
        match ip_header.protocol {
            crate::network::ipv4::protocol::TCP => {
                if header_data.len() >= TcpHeader::MIN_SIZE {
                    match TcpHeader::from_bytes(header_data) {
                        Ok(tcp_header) => (tcp_header.source_port, tcp_header.destination_port),
                        Err(_) => (0, 0),
                    }
                } else {
                    (0, 0)
                }
            }
            crate::network::ipv4::protocol::UDP => {
                if header_data.len() >= UdpHeader::SIZE {
                    match UdpHeader::from_bytes(header_data) {
                        Ok(udp_header) => (udp_header.source_port, udp_header.destination_port),
                        Err(_) => (0, 0),
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
    let mut matching_rules: Vec<(&QosRule, u32)> = Vec::new();
    
    // Process rules efficiently
    for (rule_id, rule) in &qos.rules {
        // Check source IP
        if !ip_matches(src_ip, rule.source_ip, rule.source_netmask) {
            continue;
        }
        
        // Check destination IP
        if !ip_matches(dst_ip, rule.dest_ip, rule.dest_netmask) {
            continue;
        }
        
        // Check protocol
        if rule.protocol != 0 && ip_header.protocol != rule.protocol {
            continue;
        }
        
        // Check source port
        if !port_matches(src_port, rule.source_port) {
            continue;
        }
        
        // Check destination port
        if !port_matches(dst_port, rule.dest_port) {
            continue;
        }
        
        // Check packet size
        if !size_matches(packet_size as u16, rule.min_packet_size, rule.max_packet_size) {
            continue;
        }
        
        // If we reach here, the rule matches
        matching_rules.push((rule, *rule_id));
    }
    
    // Sort by priority (lowest first)
    matching_rules.sort_by_key(|(rule, _)| rule.priority);
    
    // Return action, traffic class, and rule ID of highest priority rule, or None if no rules match
    matching_rules.first().map(|(rule, rule_id)| (rule.action, rule.traffic_class, *rule_id))
}

/// Classify a packet and return QoS action
pub fn classify_packet(
    ip_header: &Ipv4Header,
    transport_header: Option<&[u8]>,
    packet_size: usize,
) -> Option<QosAction> {
    let matched_result = match_rules(ip_header, transport_header, packet_size);
    
    if let Some((action, _traffic_class, rule_id)) = matched_result {
        let current_time_ms = crate::arch::timer::now_us() / 1000;
        
        // Handle all QoS operations in a single lock scope to prevent deadlocks
        let result = {
            let mut qos = QOS.lock();
            
            // Update statistics
            qos.stats.classified_packets += 1;
            qos.stats.classified_bytes += packet_size as u64;
            
            match action {
                QosAction::Drop => {
                    qos.stats.dropped_packets += 1;
                    qos.stats.dropped_bytes += packet_size as u64;
                }
                QosAction::Shape { .. } => {
                    qos.stats.shaped_packets += 1;
                    qos.stats.shaped_bytes += packet_size as u64;
                }
                QosAction::Police { .. } => {
                    qos.stats.policed_packets += 1;
                    qos.stats.policed_bytes += packet_size as u64;
                }
                _ => {}
            }
            
            // Handle shaping and policing
            match action {
                QosAction::Shape { .. } | QosAction::Police { .. } => {
                    if let Some(shaper) = qos.shapers.get_mut(&rule_id) {
                        if !shaper.can_consume(packet_size as u64, current_time_ms) {
                            // For policing, drop the packet
                            if let QosAction::Police { .. } = action {
                                qos.stats.dropped_packets += 1;
                                qos.stats.dropped_bytes += packet_size as u64;
                                Some(QosAction::Drop)
                            } else {
                                // For shaping, allow the packet but indicate it needs shaping
                                // The actual shaping (delaying) would be handled by the caller
                                Some(action)
                            }
                        } else {
                            Some(action)
                        }
                    } else {
                        Some(action)
                    }
                }
                _ => Some(action),
            }
        }; // End of lock scope
        
        result
    } else {
        None
    }
}

/// Set DSCP value in IP header
pub fn set_dscp_value(ip_header: &mut Ipv4Header, dscp: DscpValue) {
    // Extract current ToS field
    let tos = ip_header.type_of_service;
    
    // Clear the DSCP bits (first 6 bits)
    let cleared_tos = tos & 0x03; // Keep only ECN bits
    
    // Set new DSCP value
    let dscp_val = match dscp {
        DscpValue::BestEffort => 0,
        DscpValue::Background => 8,
        DscpValue::ExcellentEffort => 10,
        DscpValue::ControlledLoad => 12,
        DscpValue::Video => 14,
        DscpValue::Voice => 16,
        DscpValue::NetworkControl => 24,
        DscpValue::Custom(val) => val & 0x3F, // Ensure it fits in 6 bits
    };
    
    // Combine DSCP with ECN bits
    ip_header.type_of_service = (dscp_val << 2) | cleared_tos;
}

/// Get traffic class from DSCP value
pub fn dscp_to_traffic_class(dscp: u8) -> TrafficClass {
    match dscp {
        0 => TrafficClass::BestEffort,
        8 => TrafficClass::Background,
        10 => TrafficClass::ExcellentEffort,
        12 => TrafficClass::ControlledLoad,
        14 => TrafficClass::Video,
        16 => TrafficClass::Voice,
        24 => TrafficClass::NetworkControl,
        val => TrafficClass::Custom(val),
    }
}

/// Get DSCP value from traffic class
pub fn traffic_class_to_dscp(traffic_class: TrafficClass) -> DscpValue {
    match traffic_class {
        TrafficClass::BestEffort => DscpValue::BestEffort,
        TrafficClass::Background => DscpValue::Background,
        TrafficClass::ExcellentEffort => DscpValue::ExcellentEffort,
        TrafficClass::ControlledLoad => DscpValue::ControlledLoad,
        TrafficClass::Video => DscpValue::Video,
        TrafficClass::Voice => DscpValue::Voice,
        TrafficClass::NetworkControl => DscpValue::NetworkControl,
        TrafficClass::Custom(val) => DscpValue::Custom(val),
    }
}