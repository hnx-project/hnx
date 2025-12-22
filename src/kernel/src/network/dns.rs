//! DNS (Domain Name System) resolver implementation

extern crate alloc;

use crate::network::{NetworkError, Result};
use spin::Mutex;
use alloc::collections::BTreeMap;
use alloc::string::String;

/// DNS header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DnsHeader {
    /// Transaction ID
    pub id: u16,
    /// Flags
    pub flags: u16,
    /// Number of questions
    pub qd_count: u16,
    /// Number of answer resource records
    pub an_count: u16,
    /// Number of authority resource records
    pub ns_count: u16,
    /// Number of additional resource records
    pub ar_count: u16,
}

impl DnsHeader {
    /// Size of DNS header
    pub const SIZE: usize = 12;
    
    /// Create a new DNS header
    pub fn new(id: u16, flags: u16) -> Self {
        DnsHeader {
            id: id.to_be(),
            flags: flags.to_be(),
            qd_count: 1u16.to_be(), // One question
            an_count: 0u16.to_be(),
            ns_count: 0u16.to_be(),
            ar_count: 0u16.to_be(),
        }
    }
    
    /// Parse a DNS header from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::SIZE {
            return Err(NetworkError::InvalidParameter);
        }
        
        Ok(DnsHeader {
            id: u16::from_be_bytes([data[0], data[1]]),
            flags: u16::from_be_bytes([data[2], data[3]]),
            qd_count: u16::from_be_bytes([data[4], data[5]]),
            an_count: u16::from_be_bytes([data[6], data[7]]),
            ns_count: u16::from_be_bytes([data[8], data[9]]),
            ar_count: u16::from_be_bytes([data[10], data[11]]),
        })
    }
    
    /// Convert DNS header to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        
        bytes[0..2].copy_from_slice(&self.id.to_be_bytes());
        bytes[2..4].copy_from_slice(&self.flags.to_be_bytes());
        bytes[4..6].copy_from_slice(&self.qd_count.to_be_bytes());
        bytes[6..8].copy_from_slice(&self.an_count.to_be_bytes());
        bytes[8..10].copy_from_slice(&self.ns_count.to_be_bytes());
        bytes[10..12].copy_from_slice(&self.ar_count.to_be_bytes());
        
        bytes
    }
}

/// DNS question
#[derive(Debug, Clone)]
pub struct DnsQuestion {
    /// Domain name
    pub name: String,
    /// Question type
    pub q_type: u16,
    /// Question class
    pub q_class: u16,
}

impl DnsQuestion {
    /// Create a new DNS question
    pub fn new(name: &str, q_type: u16, q_class: u16) -> Self {
        DnsQuestion {
            name: String::from(name),
            q_type: q_type.to_be(),
            q_class: q_class.to_be(),
        }
    }
    
    /// Encode domain name in DNS format
    fn encode_name(&self) -> alloc::vec::Vec<u8> {
        let mut encoded = alloc::vec::Vec::new();
        let parts: alloc::vec::Vec<&str> = self.name.split('.').collect();
        
        for part in parts {
            encoded.push(part.len() as u8);
            encoded.extend_from_slice(part.as_bytes());
        }
        encoded.push(0); // Null terminator
        
        encoded
    }
    
    /// Convert DNS question to bytes
    pub fn to_bytes(&self) -> alloc::vec::Vec<u8> {
        let mut bytes = self.encode_name();
        bytes.extend_from_slice(&self.q_type.to_be_bytes());
        bytes.extend_from_slice(&self.q_class.to_be_bytes());
        bytes
    }
}

/// DNS resource record
#[derive(Debug, Clone)]
pub struct DnsResourceRecord {
    /// Domain name
    pub name: String,
    /// Resource record type
    pub rr_type: u16,
    /// Resource record class
    pub rr_class: u16,
    /// Time to live
    pub ttl: u32,
    /// Resource data length
    pub rd_length: u16,
    /// Resource data
    pub rd_data: alloc::vec::Vec<u8>,
}

/// DNS packet
#[derive(Debug, Clone)]
pub struct DnsPacket {
    /// DNS header
    pub header: DnsHeader,
    /// Questions
    pub questions: alloc::vec::Vec<DnsQuestion>,
    /// Answers
    pub answers: alloc::vec::Vec<DnsResourceRecord>,
    /// Authority records
    pub authorities: alloc::vec::Vec<DnsResourceRecord>,
    /// Additional records
    pub additionals: alloc::vec::Vec<DnsResourceRecord>,
}

impl DnsPacket {
    /// Create a new DNS packet
    pub fn new(header: DnsHeader, questions: alloc::vec::Vec<DnsQuestion>) -> Self {
        DnsPacket {
            header,
            questions,
            answers: alloc::vec::Vec::new(),
            authorities: alloc::vec::Vec::new(),
            additionals: alloc::vec::Vec::new(),
        }
    }
    
    /// Parse a DNS packet from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < DnsHeader::SIZE {
            return Err(NetworkError::InvalidParameter);
        }
        
        let header = DnsHeader::from_bytes(data)?;
        let mut offset = DnsHeader::SIZE;
        
        // Parse questions
        let mut questions = alloc::vec::Vec::new();
        for _ in 0..u16::from_be(header.qd_count) {
            let (question, new_offset) = parse_question(data, offset)?;
            questions.push(question);
            offset = new_offset;
        }
        
        // Parse answers
        let mut answers = alloc::vec::Vec::new();
        for _ in 0..u16::from_be(header.an_count) {
            let (record, new_offset) = parse_resource_record(data, offset)?;
            answers.push(record);
            offset = new_offset;
        }
        
        // Parse authorities and additionals (we'll skip these for now)
        
        Ok(DnsPacket {
            header,
            questions,
            answers,
            authorities: alloc::vec::Vec::new(),
            additionals: alloc::vec::Vec::new(),
        })
    }
    
    /// Convert DNS packet to bytes
    pub fn to_bytes(&self) -> alloc::vec::Vec<u8> {
        let mut bytes = alloc::vec::Vec::new();
        
        // Add header
        bytes.extend_from_slice(&self.header.to_bytes());
        
        // Add questions
        for question in &self.questions {
            bytes.extend_from_slice(&question.to_bytes());
        }
        
        // Add answers, authorities, and additionals (empty for queries)
        
        bytes
    }
}

/// Parse a DNS question from raw bytes
fn parse_question(data: &[u8], offset: usize) -> Result<(DnsQuestion, usize)> {
    let mut new_offset = offset;
    
    // Parse domain name
    let mut name_parts = alloc::vec::Vec::new();
    loop {
        if new_offset >= data.len() {
            return Err(NetworkError::InvalidParameter);
        }
        
        let len = data[new_offset] as usize;
        new_offset += 1;
        
        if len == 0 {
            break;
        }
        
        if new_offset + len > data.len() {
            return Err(NetworkError::InvalidParameter);
        }
        
        let part = String::from_utf8_lossy(&data[new_offset..new_offset + len]).into_owned();
        name_parts.push(part);
        new_offset += len;
    }
    
    let name = name_parts.join(".");
    
    // Parse type and class
    if new_offset + 4 > data.len() {
        return Err(NetworkError::InvalidParameter);
    }
    
    let q_type = u16::from_be_bytes([data[new_offset], data[new_offset + 1]]);
    let q_class = u16::from_be_bytes([data[new_offset + 2], data[new_offset + 3]]);
    new_offset += 4;
    
    Ok((DnsQuestion::new(&name, q_type, q_class), new_offset))
}

/// Parse a DNS resource record from raw bytes
fn parse_resource_record(data: &[u8], offset: usize) -> Result<(DnsResourceRecord, usize)> {
    let mut new_offset = offset;
    
    // Parse domain name (simplified - we'll just read it as a string for now)
    let mut name_parts = alloc::vec::Vec::new();
    loop {
        if new_offset >= data.len() {
            return Err(NetworkError::InvalidParameter);
        }
        
        let len = data[new_offset] as usize;
        new_offset += 1;
        
        if len == 0 {
            break;
        }
        
        if new_offset + len > data.len() {
            return Err(NetworkError::InvalidParameter);
        }
        
        let part = String::from_utf8_lossy(&data[new_offset..new_offset + len]).into_owned();
        name_parts.push(part);
        new_offset += len;
    }
    
    let name = name_parts.join(".");
    
    // Parse type, class, TTL, and RD length
    if new_offset + 10 > data.len() {
        return Err(NetworkError::InvalidParameter);
    }
    
    let rr_type = u16::from_be_bytes([data[new_offset], data[new_offset + 1]]);
    let rr_class = u16::from_be_bytes([data[new_offset + 2], data[new_offset + 3]]);
    let ttl = u32::from_be_bytes([data[new_offset + 4], data[new_offset + 5], data[new_offset + 6], data[new_offset + 7]]);
    let rd_length = u16::from_be_bytes([data[new_offset + 8], data[new_offset + 9]]);
    new_offset += 10;
    
    // Parse RD data
    if new_offset + rd_length as usize > data.len() {
        return Err(NetworkError::InvalidParameter);
    }
    
    let rd_data = data[new_offset..new_offset + rd_length as usize].to_vec();
    new_offset += rd_length as usize;
    
    Ok((
        DnsResourceRecord {
            name,
            rr_type,
            rr_class,
            ttl,
            rd_length,
            rd_data,
        },
        new_offset,
    ))
}

/// DNS record types
pub mod record_type {
    /// IPv4 address
    pub const A: u16 = 1;
    /// Name server
    pub const NS: u16 = 2;
    /// Canonical name
    pub const CNAME: u16 = 5;
    /// Start of authority
    pub const SOA: u16 = 6;
    /// Pointer
    pub const PTR: u16 = 12;
    /// Mail exchange
    pub const MX: u16 = 15;
    /// IPv6 address
    pub const AAAA: u16 = 28;
    /// Service
    pub const SRV: u16 = 33;
}

/// DNS classes
pub mod class {
    /// Internet
    pub const IN: u16 = 1;
}

/// DNS flags
pub mod flags {
    /// Query/response flag (0 for query, 1 for response)
    pub const QR: u16 = 0x8000;
    /// Authoritative answer
    pub const AA: u16 = 0x0400;
    /// Truncated message
    pub const TC: u16 = 0x0200;
    /// Recursion desired
    pub const RD: u16 = 0x0100;
    /// Recursion available
    pub const RA: u16 = 0x0080;
}

/// DNS cache entry
#[derive(Debug, Clone)]
struct DnsCacheEntry {
    /// IP address
    ip_address: u32,
    /// Expiration time
    expires_at: u64,
}

/// DNS cache
static DNS_CACHE: Mutex<BTreeMap<String, DnsCacheEntry>> = Mutex::new(BTreeMap::new());

/// Initialize the DNS resolver
pub fn init() {
    crate::info!("network/dns: initializing");
}

/// Send a DNS query
pub fn send_query(interface_id: u32, dns_server_ip: u32, domain_name: &str) -> Result<()> {
    // Generate a random transaction ID
    // In a real implementation, we would use a proper random number generator
    let id = 0x1234;
    
    // Create DNS header
    let header = DnsHeader::new(id, flags::RD); // Recursion desired
    
    // Create DNS question
    let question = DnsQuestion::new(domain_name, record_type::A, class::IN);
    
    // Create DNS packet
    let packet = DnsPacket::new(header, alloc::vec![question]);
    
    // Convert to bytes
    let bytes = packet.to_bytes();
    
    // Create UDP header
    let udp_header = crate::network::udp::UdpHeader::new(12345, 53, bytes.len()); // Random source port, DNS port 53
    
    // Create UDP packet
    let udp_packet = crate::network::udp::UdpPacket::new(udp_header, bytes);
    
    // Convert to bytes
    let udp_bytes = udp_packet.to_bytes();
    
    // Send the packet via IPv4
    crate::network::ipv4::send_packet(interface_id, dns_server_ip, 
                                      crate::network::ipv4::protocol::UDP, 
                                      udp_bytes)?;
    
    Ok(())
}

/// Process an incoming DNS response
pub fn process_response(interface_id: u32, src_ip: u32, data: &[u8]) -> Result<()> {
    let packet = DnsPacket::from_bytes(data)?;
    
    // Check if this is a response
    if (u16::from_be(packet.header.flags) & flags::QR) == 0 {
        return Err(NetworkError::InvalidParameter);
    }
    
    // Process answers
    for answer in &packet.answers {
        if answer.rr_type == record_type::A.to_be() && answer.rd_data.len() == 4 {
            // Extract IPv4 address
            let ip = u32::from_be_bytes([
                answer.rd_data[0],
                answer.rd_data[1],
                answer.rd_data[2],
                answer.rd_data[3],
            ]);
            
            // Add to cache
            add_to_cache(&answer.name, ip);
            
            crate::info!("network/dns: resolved {} to {:08X}", answer.name, ip);
        }
    }
    
    Ok(())
}

/// Add an entry to the DNS cache
fn add_to_cache(domain_name: &str, ip: u32) {
    let mut cache = DNS_CACHE.lock();
    
    // In a real implementation, we would use actual timestamp
    let expires_at = 0; // Placeholder
    
    let entry = DnsCacheEntry {
        ip_address: ip,
        expires_at,
    };
    
    cache.insert(String::from(domain_name), entry);
}

/// Resolve a domain name to an IP address
pub fn resolve_domain(domain_name: &str) -> Result<u32> {
    // Check cache first
    let cache = DNS_CACHE.lock();
    if let Some(entry) = cache.get(domain_name) {
        // In a real implementation, we would check expiration
        return Ok(entry.ip_address);
    }
    
    // Cache miss - in a real implementation, we would send a DNS query
    // For now, we'll return an error
    Err(NetworkError::HostUnreachable)
}