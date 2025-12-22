//! DHCP (Dynamic Host Configuration Protocol) client implementation

extern crate alloc;

use crate::network::{NetworkError, Result};
use crate::network::interface::{NetworkInterface, InterfaceFlags};

/// DHCP message op codes
pub mod opcode {
    /// Boot request
    pub const BOOT_REQUEST: u8 = 1;
    /// Boot reply
    pub const BOOT_REPLY: u8 = 2;
}

/// DHCP message types
pub mod message_type {
    /// DHCP discover
    pub const DISCOVER: u8 = 1;
    /// DHCP offer
    pub const OFFER: u8 = 2;
    /// DHCP request
    pub const REQUEST: u8 = 3;
    /// DHCP decline
    pub const DECLINE: u8 = 4;
    /// DHCP acknowledge
    pub const ACK: u8 = 5;
    /// DHCP negative acknowledge
    pub const NAK: u8 = 6;
    /// DHCP release
    pub const RELEASE: u8 = 7;
    /// DHCP inform
    pub const INFORM: u8 = 8;
}

/// DHCP options
pub mod option {
    /// Padding
    pub const PAD: u8 = 0;
    /// Subnet mask
    pub const SUBNET_MASK: u8 = 1;
    /// Router
    pub const ROUTER: u8 = 3;
    /// Domain name server
    pub const DOMAIN_NAME_SERVER: u8 = 6;
    /// Host name
    pub const HOST_NAME: u8 = 12;
    /// Domain name
    pub const DOMAIN_NAME: u8 = 15;
    /// Requested IP address
    pub const REQUESTED_IP_ADDRESS: u8 = 50;
    /// IP address lease time
    pub const IP_ADDRESS_LEASE_TIME: u8 = 51;
    /// DHCP message type
    pub const DHCP_MESSAGE_TYPE: u8 = 53;
    /// DHCP server identifier
    pub const SERVER_IDENTIFIER: u8 = 54;
    /// Parameter request list
    pub const PARAMETER_REQUEST_LIST: u8 = 55;
    /// Maximum DHCP message size
    pub const MAX_DHCP_MESSAGE_SIZE: u8 = 57;
    /// Renewal time value
    pub const RENEWAL_TIME_VALUE: u8 = 58;
    /// Rebinding time value
    pub const REBINDING_TIME_VALUE: u8 = 59;
    /// Vendor class identifier
    pub const VENDOR_CLASS_IDENTIFIER: u8 = 60;
    /// Client identifier
    pub const CLIENT_IDENTIFIER: u8 = 61;
    /// End of options
    pub const END: u8 = 255;
}

/// DHCP header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DhcpHeader {
    /// Message op code / message type
    pub op: u8,
    /// Hardware address type
    pub htype: u8,
    /// Hardware address length
    pub hlen: u8,
    /// Hop count
    pub hops: u8,
    /// Transaction ID
    pub xid: u32,
    /// Seconds elapsed since client began address acquisition
    pub secs: u16,
    /// Flags
    pub flags: u16,
    /// Client IP address
    pub ciaddr: u32,
    /// 'Your' (client) IP address
    pub yiaddr: u32,
    /// IP address of next server to use in bootstrap
    pub siaddr: u32,
    /// Relay agent IP address
    pub giaddr: u32,
    /// Client hardware address
    pub chaddr: [u8; 16],
    /// Server host name
    pub sname: [u8; 64],
    /// Boot file name
    pub file: [u8; 128],
}

impl DhcpHeader {
    /// Size of DHCP header
    pub const SIZE: usize = 236;
    
    /// Create a new DHCP header
    pub fn new(op: u8, xid: u32) -> Self {
        DhcpHeader {
            op,
            htype: 1, // Ethernet
            hlen: 6,  // MAC address length
            hops: 0,
            xid: xid.to_be(),
            secs: 0,
            flags: 0,
            ciaddr: 0,
            yiaddr: 0,
            siaddr: 0,
            giaddr: 0,
            chaddr: [0; 16],
            sname: [0; 64],
            file: [0; 128],
        }
    }
    
    /// Parse a DHCP header from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::SIZE {
            return Err(NetworkError::InvalidParameter);
        }
        
        let mut chaddr = [0u8; 16];
        let mut sname = [0u8; 64];
        let mut file = [0u8; 128];
        
        chaddr.copy_from_slice(&data[28..44]);
        sname.copy_from_slice(&data[44..108]);
        file.copy_from_slice(&data[108..236]);
        
        Ok(DhcpHeader {
            op: data[0],
            htype: data[1],
            hlen: data[2],
            hops: data[3],
            xid: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
            secs: u16::from_be_bytes([data[8], data[9]]),
            flags: u16::from_be_bytes([data[10], data[11]]),
            ciaddr: u32::from_be_bytes([data[12], data[13], data[14], data[15]]),
            yiaddr: u32::from_be_bytes([data[16], data[17], data[18], data[19]]),
            siaddr: u32::from_be_bytes([data[20], data[21], data[22], data[23]]),
            giaddr: u32::from_be_bytes([data[24], data[25], data[26], data[27]]),
            chaddr,
            sname,
            file,
        })
    }
    
    /// Convert DHCP header to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        
        bytes[0] = self.op;
        bytes[1] = self.htype;
        bytes[2] = self.hlen;
        bytes[3] = self.hops;
        bytes[4..8].copy_from_slice(&self.xid.to_be_bytes());
        bytes[8..10].copy_from_slice(&self.secs.to_be_bytes());
        bytes[10..12].copy_from_slice(&self.flags.to_be_bytes());
        bytes[12..16].copy_from_slice(&self.ciaddr.to_be_bytes());
        bytes[16..20].copy_from_slice(&self.yiaddr.to_be_bytes());
        bytes[20..24].copy_from_slice(&self.siaddr.to_be_bytes());
        bytes[24..28].copy_from_slice(&self.giaddr.to_be_bytes());
        bytes[28..44].copy_from_slice(&self.chaddr);
        bytes[44..108].copy_from_slice(&self.sname);
        bytes[108..236].copy_from_slice(&self.file);
        
        bytes
    }
}

/// DHCP packet
#[derive(Debug, Clone)]
pub struct DhcpPacket {
    /// DHCP header
    pub header: DhcpHeader,
    /// Magic cookie
    pub magic_cookie: u32,
    /// Options
    pub options: alloc::vec::Vec<u8>,
}

impl DhcpPacket {
    /// Size of DHCP fixed portion (header + magic cookie)
    pub const FIXED_SIZE: usize = DhcpHeader::SIZE + 4;
    
    /// DHCP magic cookie
    pub const MAGIC_COOKIE: u32 = 0x63825363;
    
    /// Create a new DHCP packet
    pub fn new(header: DhcpHeader, options: alloc::vec::Vec<u8>) -> Self {
        DhcpPacket {
            header,
            magic_cookie: Self::MAGIC_COOKIE.to_be(),
            options,
        }
    }
    
    /// Parse a DHCP packet from raw bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::FIXED_SIZE {
            return Err(NetworkError::InvalidParameter);
        }
        
        let header = DhcpHeader::from_bytes(data)?;
        let magic_cookie = u32::from_be_bytes([data[DhcpHeader::SIZE], 
                                               data[DhcpHeader::SIZE + 1],
                                               data[DhcpHeader::SIZE + 2],
                                               data[DhcpHeader::SIZE + 3]]);
        
        if magic_cookie != Self::MAGIC_COOKIE {
            return Err(NetworkError::InvalidParameter);
        }
        
        let options = data[Self::FIXED_SIZE..].to_vec();
        
        Ok(DhcpPacket {
            header,
            magic_cookie,
            options,
        })
    }
    
    /// Convert DHCP packet to bytes
    pub fn to_bytes(&self) -> alloc::vec::Vec<u8> {
        let mut bytes = alloc::vec::Vec::with_capacity(Self::FIXED_SIZE + self.options.len());
        
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&self.magic_cookie.to_be_bytes());
        bytes.extend_from_slice(&self.options);
        
        bytes
    }
    
    /// Add an option to the packet
    pub fn add_option(&mut self, option: u8, data: &[u8]) {
        self.options.push(option);
        self.options.push(data.len() as u8);
        self.options.extend_from_slice(data);
    }
    
    /// Find an option in the packet
    pub fn find_option(&self, option: u8) -> Option<&[u8]> {
        let mut i = 0;
        while i < self.options.len() {
            let opt = self.options[i];
            if opt == option::END {
                break;
            }
            if opt == option::PAD {
                i += 1;
                continue;
            }
            if i + 1 >= self.options.len() {
                break;
            }
            let len = self.options[i + 1] as usize;
            if i + 2 + len > self.options.len() {
                break;
            }
            if opt == option {
                return Some(&self.options[i + 2..i + 2 + len]);
            }
            i += 2 + len;
        }
        None
    }
}

/// Initialize the DHCP client
pub fn init() {
    crate::info!("network/dhcp: initializing");
}

/// Send a DHCP discover packet
pub fn send_discover(interface_id: u32) -> Result<()> {
    // Get the network interface
    let interface = match crate::network::interface::get_interface(interface_id) {
        Some(iface) => iface,
        None => return Err(NetworkError::InterfaceNotFound),
    };
    
    // Generate a random transaction ID
    // In a real implementation, we would use a proper random number generator
    let xid = 0x12345678;
    
    // Create DHCP header
    let mut header = DhcpHeader::new(opcode::BOOT_REQUEST, xid);
    header.chaddr[0..6].copy_from_slice(&interface.mac_address);
    
    // Create DHCP packet
    let mut packet = DhcpPacket::new(header, alloc::vec::Vec::new());
    
    // Add DHCP options
    packet.add_option(option::DHCP_MESSAGE_TYPE, &[message_type::DISCOVER]);
    packet.add_option(option::CLIENT_IDENTIFIER, &interface.mac_address);
    packet.add_option(option::PARAMETER_REQUEST_LIST, &[
        option::SUBNET_MASK,
        option::ROUTER,
        option::DOMAIN_NAME_SERVER,
    ]);
    packet.options.push(option::END);
    
    // Convert to bytes
    let bytes = packet.to_bytes();
    
    // Create UDP header
    let udp_header = crate::network::udp::UdpHeader::new(68, 67, bytes.len());
    
    // Create UDP packet
    let udp_packet = crate::network::udp::UdpPacket::new(udp_header, bytes);
    
    // Convert to bytes
    let udp_bytes = udp_packet.to_bytes();
    
    // Create IPv4 header
    let ipv4_header = crate::network::ipv4::Ipv4Header::new(
        interface.ip_address,
        0xFFFFFFFF, // Broadcast address
        crate::network::ipv4::protocol::UDP,
        udp_bytes.len(),
    );
    
    // Create IPv4 packet
    let ipv4_packet = crate::network::ipv4::Ipv4Packet::new(ipv4_header, udp_bytes);
    
    // Convert to bytes
    let ipv4_bytes = ipv4_packet.to_bytes();
    
    // Create Ethernet frame
    let eth_header = crate::network::ethernet::EthernetHeader::new(
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], // Broadcast MAC
        interface.mac_address,
        crate::network::ethernet::ethertype::IPV4,
    );
    
    let frame = crate::network::ethernet::EthernetFrame::new(
        eth_header,
        ipv4_bytes,
    );
    
    // Send the frame
    crate::network::ethernet::send_frame(interface_id, &frame)?;
    
    Ok(())
}

/// Process an incoming DHCP packet
pub fn process_packet(interface_id: u32, src_ip: u32, data: &[u8]) -> Result<()> {
    let packet = DhcpPacket::from_bytes(data)?;
    
    // Check if this is a DHCP reply
    if packet.header.op != opcode::BOOT_REPLY {
        return Err(NetworkError::InvalidParameter);
    }
    
    // Get message type
    let msg_type = match packet.find_option(option::DHCP_MESSAGE_TYPE) {
        Some(data) if !data.is_empty() => data[0],
        _ => return Err(NetworkError::InvalidParameter),
    };
    
    match msg_type {
        message_type::OFFER => {
            handle_offer(interface_id, &packet)?;
        }
        message_type::ACK => {
            handle_ack(interface_id, &packet)?;
        }
        message_type::NAK => {
            handle_nak(interface_id, &packet)?;
        }
        _ => {
            crate::info!("network/dhcp: unsupported message type {}", msg_type);
            return Err(NetworkError::NotSupported);
        }
    }
    
    Ok(())
}

/// Handle DHCP offer packet
fn handle_offer(interface_id: u32, packet: &DhcpPacket) -> Result<()> {
    crate::info!("network/dhcp: received DHCP offer");
    
    // Get offered IP address
    let offered_ip = packet.header.yiaddr;
    
    // Send DHCP request
    send_request(interface_id, offered_ip, packet.header.xid)?;
    
    Ok(())
}

/// Handle DHCP acknowledge packet
fn handle_ack(interface_id: u32, packet: &DhcpPacket) -> Result<()> {
    crate::info!("network/dhcp: received DHCP acknowledge");
    
    // Configure the network interface
    configure_interface(interface_id, packet)?;
    
    Ok(())
}

/// Handle DHCP negative acknowledge packet
fn handle_nak(interface_id: u32, _packet: &DhcpPacket) -> Result<()> {
    crate::info!("network/dhcp: received DHCP negative acknowledge");
    
    // In a real implementation, we would restart the DHCP process
    // For now, we'll just log the event
    
    Ok(())
}

/// Send a DHCP request packet
pub fn send_request(interface_id: u32, requested_ip: u32, xid: u32) -> Result<()> {
    // Get the network interface
    let interface = match crate::network::interface::get_interface(interface_id) {
        Some(iface) => iface,
        None => return Err(NetworkError::InterfaceNotFound),
    };
    
    // Create DHCP header
    let mut header = DhcpHeader::new(opcode::BOOT_REQUEST, xid);
    header.chaddr[0..6].copy_from_slice(&interface.mac_address);
    header.ciaddr = interface.ip_address;
    
    // Create DHCP packet
    let mut packet = DhcpPacket::new(header, alloc::vec::Vec::new());
    
    // Add DHCP options
    packet.add_option(option::DHCP_MESSAGE_TYPE, &[message_type::REQUEST]);
    packet.add_option(option::REQUESTED_IP_ADDRESS, &requested_ip.to_be_bytes());
    packet.add_option(option::SERVER_IDENTIFIER, &packet.header.siaddr.to_be_bytes());
    packet.add_option(option::CLIENT_IDENTIFIER, &interface.mac_address);
    packet.options.push(option::END);
    
    // Convert to bytes
    let bytes = packet.to_bytes();
    
    // Create UDP header
    let udp_header = crate::network::udp::UdpHeader::new(68, 67, bytes.len());
    
    // Create UDP packet
    let udp_packet = crate::network::udp::UdpPacket::new(udp_header, bytes);
    
    // Convert to bytes
    let udp_bytes = udp_packet.to_bytes();
    
    // Create IPv4 header
    let ipv4_header = crate::network::ipv4::Ipv4Header::new(
        interface.ip_address,
        0xFFFFFFFF, // Broadcast address
        crate::network::ipv4::protocol::UDP,
        udp_bytes.len(),
    );
    
    // Create IPv4 packet
    let ipv4_packet = crate::network::ipv4::Ipv4Packet::new(ipv4_header, udp_bytes);
    
    // Convert to bytes
    let ipv4_bytes = ipv4_packet.to_bytes();
    
    // Create Ethernet frame
    let eth_header = crate::network::ethernet::EthernetHeader::new(
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF], // Broadcast MAC
        interface.mac_address,
        crate::network::ethernet::ethertype::IPV4,
    );
    
    let frame = crate::network::ethernet::EthernetFrame::new(
        eth_header,
        ipv4_bytes,
    );
    
    // Send the frame
    crate::network::ethernet::send_frame(interface_id, &frame)?;
    
    Ok(())
}

/// Configure network interface based on DHCP response
fn configure_interface(interface_id: u32, packet: &DhcpPacket) -> Result<()> {
    // Get the network interface
    let mut interface = match crate::network::interface::get_interface(interface_id) {
        Some(iface) => iface,
        None => return Err(NetworkError::InterfaceNotFound),
    };
    
    // Set IP address
    interface.ip_address = packet.header.yiaddr;
    
    // Set subnet mask if provided
    if let Some(mask_data) = packet.find_option(option::SUBNET_MASK) {
        if mask_data.len() == 4 {
            let mask = u32::from_be_bytes([mask_data[0], mask_data[1], mask_data[2], mask_data[3]]);
            interface.netmask = mask;
        }
    }
    
    // Set gateway if provided
    if let Some(router_data) = packet.find_option(option::ROUTER) {
        if router_data.len() >= 4 {
            let gateway = u32::from_be_bytes([router_data[0], router_data[1], router_data[2], router_data[3]]);
            interface.gateway = gateway;
        }
    }
    
    // Add route for the network
    if interface.netmask != 0 {
        let network = interface.ip_address & interface.netmask;
        crate::network::ipv4::add_route(network, interface.netmask, 0, interface_id)?;
    }
    
    // Add default route if gateway is provided
    if interface.gateway != 0 {
        crate::network::ipv4::add_route(0, 0, interface.gateway, interface_id)?;
    }
    
    // In a real implementation, we would update the global interface table
    // For now, we'll just log the configuration
    
    crate::info!("network/dhcp: configured interface {} with IP {:08X}, netmask {:08X}, gateway {:08X}",
                 interface.name, interface.ip_address, interface.netmask, interface.gateway);
    
    Ok(())
}