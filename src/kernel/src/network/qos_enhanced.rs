//! Enhanced Quality of Service (QoS) implementation for the HNX kernel with IPv6 support
//!
//! This module extends the existing QoS implementation to support IPv6 packets
//! and adds more sophisticated traffic management capabilities.

extern crate alloc;

use crate::network::{NetworkError, Result};
use crate::network::ipv4::Ipv4Header;
use crate::network::ipv6::Ipv6Header;
use crate::network::tcp::TcpHeader;
use crate::network::udp::UdpHeader;
use spin::Mutex;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// Import alloc::vec for vec! macro
use alloc::vec;

// Re-export existing QoS types
pub use super::qos::{
    QosAction, TrafficClass, DscpValue, QosRule, QosStats, TrafficShaper,
    enable, disable, is_enabled, add_rule, create_rule, remove_rule, list_rules,
    get_stats, reset_stats, set_dscp_value, dscp_to_traffic_class, traffic_class_to_dscp
};

/// Enhanced QoS rule for IPv6 traffic classification
#[derive(Debug, Clone)]
pub struct QosRuleV6 {
    /// Rule ID (unique identifier)
    pub id: u32,
    /// Rule priority (lower values have higher priority)
    pub priority: u32,
    /// Action to take when rule matches
    pub action: QosAction,
    /// Source IPv6 address
    pub source_ipv6: [u8; 16],
    /// Source IPv6 prefix length
    pub source_prefix_len: u8,
    /// Destination IPv6 address
    pub dest_ipv6: [u8; 16],
    /// Destination IPv6 prefix length
    pub dest_prefix_len: u8,
    /// Next header (protocol) specification
    pub next_header: u8,
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

/// Enhanced QoS statistics with IPv6 counters
#[derive(Debug, Clone)]
pub struct QosStatsV6 {
    /// Number of IPv4 packets classified
    pub ipv4_classified_packets: u64,
    /// Number of IPv4 bytes classified
    pub ipv4_classified_bytes: u64,
    /// Number of IPv4 packets dropped
    pub ipv4_dropped_packets: u64,
    /// Number of IPv4 bytes dropped
    pub ipv4_dropped_bytes: u64,
    /// Number of IPv4 packets shaped
    pub ipv4_shaped_packets: u64,
    /// Number of IPv4 bytes shaped
    pub ipv4_shaped_bytes: u64,
    /// Number of IPv4 packets policed
    pub ipv4_policed_packets: u64,
    /// Number of IPv4 bytes policed
    pub ipv4_policed_bytes: u64,
    /// Number of IPv6 packets classified
    pub ipv6_classified_packets: u64,
    /// Number of IPv6 bytes classified
    pub ipv6_classified_bytes: u64,
    /// Number of IPv6 packets dropped
    pub ipv6_dropped_packets: u64,
    /// Number of IPv6 bytes dropped
    pub ipv6_dropped_bytes: u64,
    /// Number of IPv6 packets shaped
    pub ipv6_shaped_packets: u64,
    /// Number of IPv6 bytes shaped
    pub ipv6_shaped_bytes: u64,
    /// Number of IPv6 packets policed
    pub ipv6_policed_packets: u64,
    /// Number of IPv6 bytes policed
    pub ipv6_policed_bytes: u64,
}

impl QosStatsV6 {
    /// Create new empty QoS statistics
    const fn new() -> Self {
        QosStatsV6 {
            ipv4_classified_packets: 0,
            ipv4_classified_bytes: 0,
            ipv4_dropped_packets: 0,
            ipv4_dropped_bytes: 0,
            ipv4_shaped_packets: 0,
            ipv4_shaped_bytes: 0,
            ipv4_policed_packets: 0,
            ipv4_policed_bytes: 0,
            ipv6_classified_packets: 0,
            ipv6_classified_bytes: 0,
            ipv6_dropped_packets: 0,
            ipv6_dropped_bytes: 0,
            ipv6_shaped_packets: 0,
            ipv6_shaped_bytes: 0,
            ipv6_policed_packets: 0,
            ipv6_policed_bytes: 0,
        }
    }
}

/// Enhanced global QoS state with IPv6 support
struct QosStateV6 {
    /// Active IPv4 QoS rules
    ipv4_rules: BTreeMap<u32, super::qos::QosRule>,
    /// Active IPv6 QoS rules
    ipv6_rules: BTreeMap<u32, QosRuleV6>,
    /// QoS statistics
    stats: QosStatsV6,
    /// Next rule ID to assign
    next_rule_id: u32,
    /// Whether QoS is enabled
    enabled: bool,
    /// Traffic shapers by rule ID for IPv4
    ipv4_shapers: BTreeMap<u32, super::qos::TrafficShaper>,
    /// Traffic shapers by rule ID for IPv6
    ipv6_shapers: BTreeMap<u32, super::qos::TrafficShaper>,
}

impl QosStateV6 {
    /// Create new QoS state
    const fn new() -> Self {
        QosStateV6 {
            ipv4_rules: BTreeMap::new(),
            ipv6_rules: BTreeMap::new(),
            stats: QosStatsV6::new(),
            next_rule_id: 1,
            enabled: false,
            ipv4_shapers: BTreeMap::new(),
            ipv6_shapers: BTreeMap::new(),
        }
    }
}

/// Enhanced global QoS instance
static QOS_V6: Mutex<QosStateV6> = Mutex::new(QosStateV6::new());

/// Initialize the enhanced QoS subsystem
pub fn init_v6() {
    crate::info!("network/qos_v6: initializing");
    
    let mut qos = QOS_V6.lock();
    *qos = QosStateV6::new();
    
    crate::info!("network/qos_v6: initialized");
}

/// Add a new IPv6 QoS rule
pub fn add_rule_v6(rule: QosRuleV6) -> Result<u32> {
    let mut qos = QOS_V6.lock();
    
    // Check if rule ID already exists
    if qos.ipv6_rules.contains_key(&rule.id) {
        return Err(NetworkError::AddressInUse);
    }
    
    // If the rule has shaping or policing, create a traffic shaper
    let rule_id = rule.id;
    match rule.action {
        QosAction::Shape { rate_bps } | QosAction::Police { rate_bps } => {
            let shaper = super::qos::TrafficShaper::new(rate_bps);
            qos.ipv6_shapers.insert(rule_id, shaper);
        }
        _ => {}
    }
    
    qos.ipv6_rules.insert(rule_id, rule);
    
    crate::info!("network/qos_v6: added rule with ID {}", rule_id);
    
    Ok(rule_id)
}

/// Create and add a new IPv6 QoS rule with auto-assigned ID
pub fn create_rule_v6(
    priority: u32,
    action: QosAction,
    source_ipv6: [u8; 16],
    source_prefix_len: u8,
    dest_ipv6: [u8; 16],
    dest_prefix_len: u8,
    next_header: u8,
    source_port: u16,
    dest_port: u16,
    min_packet_size: u16,
    max_packet_size: u16,
    traffic_class: TrafficClass,
) -> Result<u32> {
    let mut qos = QOS_V6.lock();
    
    let rule_id = qos.next_rule_id;
    qos.next_rule_id += 1;
    
    let rule = QosRuleV6 {
        id: rule_id,
        priority,
        action,
        source_ipv6,
        source_prefix_len,
        dest_ipv6,
        dest_prefix_len,
        next_header,
        source_port,
        dest_port,
        min_packet_size,
        max_packet_size,
        traffic_class,
    };
    
    // If the rule has shaping or policing, create a traffic shaper
    match action {
        QosAction::Shape { rate_bps } | QosAction::Police { rate_bps } => {
            let shaper = super::qos::TrafficShaper::new(rate_bps);
            qos.ipv6_shapers.insert(rule_id, shaper);
        }
        _ => {}
    }
    
    qos.ipv6_rules.insert(rule_id, rule);
    
    crate::info!("network/qos_v6: created rule with ID {}", rule_id);
    
    Ok(rule_id)
}

/// Remove an IPv6 QoS rule by ID
pub fn remove_rule_v6(rule_id: u32) -> Result<()> {
    let mut qos = QOS_V6.lock();
    
    if qos.ipv6_rules.remove(&rule_id).is_some() {
        // Also remove associated shaper if it exists
        qos.ipv6_shapers.remove(&rule_id);
        crate::info!("network/qos_v6: removed rule with ID {}", rule_id);
        Ok(())
    } else {
        Err(NetworkError::InvalidParameter)
    }
}

/// List all IPv6 QoS rules
pub fn list_rules_v6() -> Vec<QosRuleV6> {
    let qos = QOS_V6.lock();
    qos.ipv6_rules.values().cloned().collect()
}

/// Get enhanced QoS statistics
pub fn get_stats_v6() -> QosStatsV6 {
    let qos = QOS_V6.lock();
    qos.stats.clone()
}

/// Reset enhanced QoS statistics
pub fn reset_stats_v6() {
    let mut qos = QOS_V6.lock();
    qos.stats = QosStatsV6::new();
}

/// Check if an IPv6 address matches an IPv6 specification with prefix length
fn ipv6_matches(ip: [u8; 16], target_ip: [u8; 16], prefix_len: u8) -> bool {
    // Compare only the network portion based on prefix length
    let bytes_to_compare = (prefix_len as usize) / 8;
    let bits_to_compare = (prefix_len as usize) % 8;
    
    // Compare full bytes
    if bytes_to_compare > 0 && ip[..bytes_to_compare] != target_ip[..bytes_to_compare] {
        return false;
    }
    
    // Compare remaining bits if any
    if bits_to_compare > 0 && bytes_to_compare < 16 {
        let mask = !((1 << (8 - bits_to_compare)) - 1);
        if (ip[bytes_to_compare] & mask) != (target_ip[bytes_to_compare] & mask) {
            return false;
        }
    }
    
    true
}

/// Match an IPv6 packet against QoS rules
fn match_rules_v6(
    ipv6_header: &Ipv6Header,
    transport_header: Option<&[u8]>,
    packet_size: usize,
) -> Option<(QosAction, TrafficClass, u32)> {
    let qos = QOS_V6.lock();
    
    // If QoS is disabled, don't classify packets
    if !qos.enabled {
        return None;
    }
    
    // Extract port information if available
    let (src_port, dst_port) = if let Some(header_data) = transport_header {
        match ipv6_header.next_header {
            crate::network::ipv6::next_header::TCP => {
                if header_data.len() >= TcpHeader::MIN_SIZE {
                    match TcpHeader::from_bytes(header_data) {
                        Ok(tcp_header) => (tcp_header.source_port, tcp_header.destination_port),
                        Err(_) => (0, 0),
                    }
                } else {
                    (0, 0)
                }
            }
            crate::network::ipv6::next_header::UDP => {
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
    let mut matching_rules: Vec<(&QosRuleV6, u32)> = Vec::new();
    
    // Process rules efficiently
    for (rule_id, rule) in &qos.ipv6_rules {
        // Check source IPv6
        if !ipv6_matches(ipv6_header.source_addr, rule.source_ipv6, rule.source_prefix_len) {
            continue;
        }
        
        // Check destination IPv6
        if !ipv6_matches(ipv6_header.destination_addr, rule.dest_ipv6, rule.dest_prefix_len) {
            continue;
        }
        
        // Check next header (protocol)
        if rule.next_header != 0 && ipv6_header.next_header != rule.next_header {
            continue;
        }
        
        // Check source port
        if rule.source_port != 0 && src_port != rule.source_port {
            continue;
        }
        
        // Check destination port
        if rule.dest_port != 0 && dst_port != rule.dest_port {
            continue;
        }
        
        // Check packet size
        let packet_size_u16 = packet_size as u16;
        if (rule.min_packet_size != 0 && packet_size_u16 < rule.min_packet_size) ||
           (rule.max_packet_size != 0 && packet_size_u16 > rule.max_packet_size) {
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

/// Classify an IPv6 packet and return QoS action
pub fn classify_packet_v6(
    ipv6_header: &Ipv6Header,
    transport_header: Option<&[u8]>,
    packet_size: usize,
) -> Option<QosAction> {
    let matched_result = match_rules_v6(ipv6_header, transport_header, packet_size);
    
    if let Some((action, _traffic_class, rule_id)) = matched_result {
        let current_time_ms = crate::arch::timer::now_us() / 1000;
        
        // Handle all QoS operations in a single lock scope to prevent deadlocks
        let result = {
            let mut qos = QOS_V6.lock();
            
            // Update statistics
            qos.stats.ipv6_classified_packets += 1;
            qos.stats.ipv6_classified_bytes += packet_size as u64;
            
            match action {
                QosAction::Drop => {
                    qos.stats.ipv6_dropped_packets += 1;
                    qos.stats.ipv6_dropped_bytes += packet_size as u64;
                }
                QosAction::Shape { .. } => {
                    qos.stats.ipv6_shaped_packets += 1;
                    qos.stats.ipv6_shaped_bytes += packet_size as u64;
                }
                QosAction::Police { .. } => {
                    qos.stats.ipv6_policed_packets += 1;
                    qos.stats.ipv6_policed_bytes += packet_size as u64;
                }
                _ => {}
            }
            
            // Handle shaping and policing
            match action {
                QosAction::Shape { .. } | QosAction::Police { .. } => {
                    if let Some(shaper) = qos.ipv6_shapers.get_mut(&rule_id) {
                        if !shaper.can_consume(packet_size as u64, current_time_ms) {
                            // For policing, drop the packet
                            if let QosAction::Police { .. } = action {
                                qos.stats.ipv6_dropped_packets += 1;
                                qos.stats.ipv6_dropped_bytes += packet_size as u64;
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

/// Set Traffic Class value in IPv6 header
pub fn set_traffic_class_value(ipv6_header: &mut Ipv6Header, traffic_class: TrafficClass) {
    // Get DSCP value from traffic class
    let dscp = match traffic_class_to_dscp(traffic_class) {
        DscpValue::BestEffort => 0,
        DscpValue::Background => 8,
        DscpValue::ExcellentEffort => 10,
        DscpValue::ControlledLoad => 12,
        DscpValue::Video => 14,
        DscpValue::Voice => 16,
        DscpValue::NetworkControl => 24,
        DscpValue::Custom(val) => val & 0x3F, // Ensure it fits in 6 bits
    };
    
    // Extract current Traffic Class and Flow Label
    let current_value = ipv6_header.version_traffic_class_flow_label;
    
    // Clear the Traffic Class bits (bits 20-27)
    let cleared_value = current_value & 0xF03FFFFF; // Clear bits 20-27
    
    // Set new Traffic Class value (shifted to position 20-27)
    let tc_value = ((dscp as u32) << 2) << 20; // DSCP is in upper 6 bits of Traffic Class
    
    // Combine with cleared value
    ipv6_header.version_traffic_class_flow_label = cleared_value | tc_value;
}

/// Hierarchical Token Bucket (HTB) scheduler implementation
pub mod htb_scheduler {
    extern crate alloc;
    
    use spin::Mutex;
    use alloc::collections::BTreeMap;
    use alloc::vec::Vec;
    
    /// HTB class representing a node in the hierarchy
    #[derive(Debug, Clone)]
    pub struct HtbClass {
        pub class_id: u32,
        pub parent_id: Option<u32>,
        pub rate: u64,      // Bytes per second
        pub ceil: u64,      // Max burst rate (bytes per second)
        pub buffer: u32,    // Buffer size in bytes
        pub cbuffer: u32,   // Ceiling buffer size in bytes
        pub quantum: u32,   // Quantum for round-robin scheduling
        pub level: u8,      // Hierarchy level (0 = leaf)
        pub prio: u8,       // Priority (0 = highest)
        pub children: Vec<u32>, // Child class IDs
    }
    
    /// HTB scheduler state
    struct HtbSchedulerState {
        classes: BTreeMap<u32, HtbClass>,
        root_class: Option<u32>,
    }
    
    impl HtbSchedulerState {
        const fn new() -> Self {
            HtbSchedulerState {
                classes: BTreeMap::new(),
                root_class: None,
            }
        }
    }
    
    /// Global HTB scheduler instance
    static HTB_SCHEDULER: Mutex<HtbSchedulerState> = Mutex::new(HtbSchedulerState::new());
    
    /// Initialize the HTB scheduler
    pub fn init() {
        crate::info!("network/qos/htb: initializing");
    }
    
    /// Add an HTB class
    pub fn add_class(htb_class: HtbClass) -> Result<(), &'static str> {
        let mut scheduler = HTB_SCHEDULER.lock();
        
        // Check if class ID already exists
        if scheduler.classes.contains_key(&htb_class.class_id) {
            return Err("Class ID already exists");
        }
        
        // If this is the first class or explicitly marked as root, set as root
        if scheduler.root_class.is_none() {
            scheduler.root_class = Some(htb_class.class_id);
        }
        
        // Add to parent's children list if parent exists
        if let Some(parent_id) = htb_class.parent_id {
            if let Some(parent) = scheduler.classes.get_mut(&parent_id) {
                parent.children.push(htb_class.class_id);
            }
        }
        
        scheduler.classes.insert(htb_class.class_id, htb_class);
        Ok(())
    }
    
    /// Remove an HTB class
    pub fn remove_class(class_id: u32) -> Result<(), &'static str> {
        let mut scheduler = HTB_SCHEDULER.lock();
        
        // Check if class exists
        if !scheduler.classes.contains_key(&class_id) {
            return Err("Class ID not found");
        }
        
        // Remove from parent's children list if parent exists
        let parent_id = scheduler.classes[&class_id].parent_id;
        if let Some(parent_id) = parent_id {
            if let Some(parent) = scheduler.classes.get_mut(&parent_id) {
                parent.children.retain(|&child_id| child_id != class_id);
            }
        }
        
        // If this was the root class, clear root
        if scheduler.root_class == Some(class_id) {
            scheduler.root_class = None;
        }
        
        scheduler.classes.remove(&class_id);
        Ok(())
    }
    
    /// Get an HTB class
    pub fn get_class(class_id: u32) -> Option<HtbClass> {
        let scheduler = HTB_SCHEDULER.lock();
        scheduler.classes.get(&class_id).cloned()
    }
    
    /// List all HTB classes
    pub fn list_classes() -> Vec<HtbClass> {
        let scheduler = HTB_SCHEDULER.lock();
        scheduler.classes.values().cloned().collect()
    }
}

/// Stochastic Fairness Queueing (SFQ) implementation
pub mod sfq_scheduler {
    extern crate alloc;
    
    use spin::Mutex;
    use alloc::collections::VecDeque;
    use alloc::vec::Vec;
    
    /// SFQ flow identifier
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub struct FlowId {
        pub src_ip: [u8; 16],  // IPv6 format to accommodate both IPv4 and IPv6
        pub dst_ip: [u8; 16],
        pub src_port: u16,
        pub dst_port: u16,
        pub protocol: u8,
    }
    
    /// SFQ queue
    #[derive(Debug)]
    pub struct SfqQueue {
        pub packets: VecDeque<alloc::vec::Vec<u8>>, // Packet data
        pub flow_id: FlowId,
        pub last_activity: u64, // Timestamp
    }
    
    /// SFQ scheduler state
    pub struct SfqSchedulerState {
        queues: [Option<SfqQueue>; 128],
        hash_table: alloc::collections::BTreeMap<FlowId, usize>, // FlowId -> queue index
        current_queue: usize, // Round-robin pointer
        max_depth: usize,
    }
    
    impl SfqSchedulerState {
        pub const fn new() -> Self {
            // Create array with None values
            const NONE_QUEUE: Option<SfqQueue> = None;
            SfqSchedulerState {
                queues: [NONE_QUEUE; 128],
                hash_table: alloc::collections::BTreeMap::new(),
                current_queue: 0,
                max_depth: 128,
            }
        }
    }
    
    /// Global SFQ scheduler instance
static SFQ_SCHEDULER: Mutex<SfqSchedulerState> = Mutex::new(SfqSchedulerState::new());
    
    /// Initialize the SFQ scheduler
    pub fn init() {
        crate::info!("network/qos/sfq: initializing");
    }
    
    /// Hash a flow ID to determine queue index
    fn hash_flow(flow_id: &FlowId, max_queues: usize) -> usize {
        let mut hash = 0u64;
        
        // Simple hash function combining all flow components
        for &byte in &flow_id.src_ip {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
        for &byte in &flow_id.dst_ip {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
        hash = hash.wrapping_mul(31).wrapping_add(flow_id.src_port as u64);
        hash = hash.wrapping_mul(31).wrapping_add(flow_id.dst_port as u64);
        hash = hash.wrapping_mul(31).wrapping_add(flow_id.protocol as u64);
        
        (hash as usize) % max_queues
    }
    
    /// Add a packet to the SFQ scheduler
    pub fn enqueue_packet(flow_id: FlowId, packet: alloc::vec::Vec<u8>) -> Result<(), &'static str> {
        let mut scheduler = SFQ_SCHEDULER.lock();
        let timestamp = crate::arch::timer::now_us() / 1000; // Current time in ms
        
        // Hash the flow to find the queue index (fixed max_queues to 128)
        let queue_index = hash_flow(&flow_id, 128);
        
        // Check if this flow already has a queue
        let queue_pos = if let Some(&pos) = scheduler.hash_table.get(&flow_id) {
            pos
        } else {
            // Find an empty queue slot or reuse the current round-robin position
            let mut pos = queue_index;
            for _ in 0..128 {
                if scheduler.queues[pos].is_none() {
                    break;
                }
                pos = (pos + 1) % 128;
            }
            
            // If we found an empty slot, use it
            if scheduler.queues[pos].is_none() {
                pos
            } else {
                // Reuse the current round-robin position
                scheduler.current_queue
            }
        };
        
        // Store max_depth in a local variable to avoid borrowing issues
        let max_depth = scheduler.max_depth;
        
        // Create or update the queue
        match &mut scheduler.queues[queue_pos] {
            Some(queue) => {
                // Update existing queue
                queue.flow_id = flow_id;
                queue.last_activity = timestamp;
                
                // Check if queue is full
                if queue.packets.len() >= max_depth {
                    // Drop oldest packet
                    queue.packets.pop_front();
                }
                
                queue.packets.push_back(packet);
            }
            None => {
                // Create new queue
                let mut packets = VecDeque::new();
                packets.push_back(packet);
                
                let queue = SfqQueue {
                    packets,
                    flow_id,
                    last_activity: timestamp,
                };
                
                scheduler.queues[queue_pos] = Some(queue);
                scheduler.hash_table.insert(flow_id, queue_pos);
            }
        }
        
        Ok(())
    }
    
    /// Dequeue a packet from the SFQ scheduler
    pub fn dequeue_packet() -> Option<(FlowId, alloc::vec::Vec<u8>)> {
        let mut scheduler = SFQ_SCHEDULER.lock();
        
        // Store current queue index to avoid borrowing issues
        let mut current_queue_idx = scheduler.current_queue;
        
        // Try to dequeue from the current queue and advance round-robin pointer
        for _ in 0..128 {
            // Get a reference to the queue without holding a mutable reference to scheduler
            let queue_ref = &mut scheduler.queues[current_queue_idx];
            
            if let Some(queue) = queue_ref {
                if let Some(packet) = queue.packets.pop_front() {
                    let flow_id = queue.flow_id;
                    
                    // If queue is now empty, remove it
                    if queue.packets.is_empty() {
                        scheduler.hash_table.remove(&flow_id);
                        scheduler.queues[current_queue_idx] = None;
                    }
                    
                    // Advance round-robin pointer
                    scheduler.current_queue = (current_queue_idx + 1) % 128;
                    
                    return Some((flow_id, packet));
                }
            }
            
            // Advance round-robin pointer for next iteration
            current_queue_idx = (current_queue_idx + 1) % 128;
        }
        
        None
    }
    
    /// Get the current queue depths
    pub fn get_queue_depths() -> Vec<(usize, usize)> {
        let scheduler = SFQ_SCHEDULER.lock();
        let mut depths = Vec::new();
        
        for (index, queue) in scheduler.queues.iter().enumerate() {
            if let Some(q) = queue {
                depths.push((index, q.packets.len()));
            }
        }
        
        depths
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
        assert!(ipv6_matches(addr1, addr2, 128));
        
        // Test prefix match
        let network: [u8; 16] = [0x20, 0x01, 0x0d, 0xb8, 0x00, 0x00, 0x00, 0x00, 
                                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let addr_in_subnet: [u8; 16] = [0x20, 0x01, 0x0d, 0xb8, 0x00, 0x00, 0x00, 0x00, 
                                       0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05];
        assert!(ipv6_matches(network, addr_in_subnet, 64));
        
        // Test mismatch
        let addr3: [u8; 16] = [0x20, 0x01, 0x0d, 0xb8, 0x00, 0x00, 0x00, 0x00, 
                              0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02];
        assert!(!ipv6_matches(network, addr3, 128));
    }
    
    #[test]
    fn test_qos_v6_rule_creation() {
        // Test creating an IPv6 QoS rule
        let source_ipv6: [u8; 16] = [0; 16];
        let dest_ipv6: [u8; 16] = [0; 16];
        let next_header = 6; // TCP
        let source_port = 80;
        let dest_port = 0; // Any
        let traffic_class = TrafficClass::Video;
        
        let rule_id = create_rule_v6(
            100,
            QosAction::Allow { priority: 1 },
            source_ipv6,
            128,
            dest_ipv6,
            128,
            next_header,
            source_port,
            dest_port,
            0,
            0,
            traffic_class,
        );
        
        assert!(rule_id.is_ok());
        
        // Verify the rule was added
        let rules = list_rules_v6();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, rule_id.unwrap());
    }
}