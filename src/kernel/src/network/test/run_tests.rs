//! Test runner for network functionality
//!
//! This module runs all network tests to verify that the network subsystem
//! is functioning correctly.

extern crate alloc;

use crate::network::test::test_firewall;
use crate::network::test::test_qos;
use crate::network::test::test_network_monitoring;
use crate::network::test::test_tcp_send_recv;
use crate::network::test::test_tcp_accept;
use crate::network::test::test_socket_nonblocking;
use crate::network::test::integration_test;
use crate::network::test::integration_test_send_recv;
use crate::network::test::integration_test_accept;
use crate::network::test::enhanced_tests;

/// Run all network tests
pub fn run_all_network_tests() -> Result<(), &'static str> {
    crate::info!("network/test_runner: starting all network tests");
    
    // Run firewall tests
    crate::info!("network/test_runner: running firewall tests");
    test_firewall::run_all_tests();
    crate::info!("network/test_runner: firewall tests completed");
    
    // Run QoS tests
    crate::info!("network/test_runner: running QoS tests");
    test_qos::run_all_tests();
    crate::info!("network/test_runner: QoS tests completed");
    
    // Run monitoring tests
    crate::info!("network/test_runner: running network monitoring tests");
    match test_network_monitoring::test_network_monitoring() {
        Ok(_) => crate::info!("network/test_runner: network monitoring test passed"),
        Err(e) => {
            crate::error!("network/test_runner: network monitoring test failed: {:?}", e);
            return Err("network monitoring test failed");
        }
    }
    crate::info!("network/test_runner: network monitoring tests completed");
    
    // Run TCP send/recv tests
    crate::info!("network/test_runner: running TCP send/recv tests");
    match test_tcp_send_recv::test_tcp_send_recv() {
        Ok(_) => crate::info!("network/test_runner: TCP send/recv test passed"),
        Err(e) => {
            crate::error!("network/test_runner: TCP send/recv test failed: {:?}", e);
            return Err("TCP send/recv test failed");
        }
    }
    
    // Run TCP accept tests
    crate::info!("network/test_runner: running TCP accept tests");
    match test_tcp_accept::test_tcp_accept() {
        Ok(_) => crate::info!("network/test_runner: TCP accept test passed"),
        Err(e) => {
            crate::error!("network/test_runner: TCP accept test failed: {:?}", e);
            return Err("TCP accept test failed");
        }
    }
    
    // Run socket non-blocking tests
    crate::info!("network/test_runner: running socket non-blocking tests");
    match test_socket_nonblocking::test_set_nonblocking() {
        Ok(_) => crate::info!("network/test_runner: socket non-blocking test passed"),
        Err(e) => {
            crate::error!("network/test_runner: socket non-blocking test failed: {:?}", e);
            return Err("socket non-blocking test failed");
        }
    }
    
    // Run integration tests
    crate::info!("network/test_runner: running main integration test");
    match integration_test::integration_test_network() {
        Ok(_) => crate::info!("network/test_runner: main integration test passed"),
        Err(e) => {
            crate::error!("network/test_runner: main integration test failed: {:?}", e);
            return Err("main integration test failed");
        }
    }
    
    // Run send/recv integration tests
    crate::info!("network/test_runner: running send/recv integration test");
    match integration_test_send_recv::integration_test_tcp_send_recv() {
        Ok(_) => crate::info!("network/test_runner: send/recv integration test passed"),
        Err(e) => {
            crate::error!("network/test_runner: send/recv integration test failed: {:?}", e);
            return Err("send/recv integration test failed");
        }
    }
    
    // Run accept integration tests
    crate::info!("network/test_runner: running accept integration test");
    match integration_test_accept::integration_test_tcp_accept() {
        Ok(_) => crate::info!("network/test_runner: accept integration test passed"),
        Err(e) => {
            crate::error!("network/test_runner: accept integration test failed: {:?}", e);
            return Err("accept integration test failed");
        }
    }
    
    // Run enhanced network tests
    crate::info!("network/test_runner: running enhanced network tests");
    match enhanced_tests::run_all_enhanced_network_tests() {
        Ok(_) => crate::info!("network/test_runner: enhanced network tests passed"),
        Err(e) => {
            crate::error!("network/test_runner: enhanced network tests failed: {:?}", e);
            return Err("enhanced network tests failed");
        }
    }
    
    crate::info!("network/test_runner: all network tests completed successfully");
    Ok(())
}