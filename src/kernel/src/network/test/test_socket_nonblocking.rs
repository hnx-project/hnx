//! Test for socket non-blocking I/O functionality

extern crate alloc;

use crate::network::test::{socket, NetworkError};
use crate::network::test::socket::{SocketDomain, SocketType, SocketProtocol, set_nonblocking};

/// Test setting socket to non-blocking mode
pub fn test_set_nonblocking() -> Result<(), NetworkError> {
    // Create a socket
    let socket_id = socket::socket(
        SocketDomain::INET,
        SocketType::STREAM,
        SocketProtocol::TCP,
    )?;
    
    // Set socket to non-blocking mode
    set_nonblocking(socket_id, true)?;
    
    // Try to receive data from the non-blocking socket (should return WouldBlock)
    let mut buffer = [0u8; 1024];
    match socket::recv(socket_id, &mut buffer) {
        Err(NetworkError::WouldBlock) => {
            crate::info!("test_socket_nonblocking: correctly returned WouldBlock for non-blocking socket");
        },
        Ok(_) => {
            crate::info!("test_socket_nonblocking: unexpectedly received data");
            return Err(NetworkError::InvalidParameter);
        },
        Err(e) => {
            crate::info!("test_socket_nonblocking: unexpected error: {:?}", e);
            return Err(e);
        }
    }
    
    // Set socket back to blocking mode
    set_nonblocking(socket_id, false)?;
    
    // Close the socket
    socket::close(socket_id)?;
    
    crate::info!("test_socket_nonblocking: passed");
    Ok(())
}