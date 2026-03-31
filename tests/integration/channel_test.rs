//! Channel Integration Tests
//!
//! Tests for state channel lifecycle and micro-transactions.

use nexa_net::economy::{ChannelManager, Channel, ChannelConfig};
use nexa_net::types::Did;
use std::time::Duration;

/// Create a test DID
fn test_did(name: &str) -> Did {
    Did::new(&format!("did:nexa:{}", name))
}

#[test]
fn test_channel_lifecycle() {
    let mut manager = ChannelManager::new();
    
    let party_a = test_did("alice");
    let party_b = test_did("bob");
    
    // Open channel
    let channel = manager.open(party_a.clone(), party_b.clone(), 1000, 500).unwrap();
    assert_eq!(channel.balance_a, 1000);
    assert_eq!(channel.balance_b, 500);
    
    // Verify channel is listed
    let open_channels = manager.list_open();
    assert_eq!(open_channels.len(), 1);
}

#[test]
fn test_channel_transfers() {
    let mut manager = ChannelManager::new();
    
    let party_a = test_did("alice");
    let party_b = test_did("bob");
    
    // Open channel
    manager.open(party_a.clone(), party_b.clone(), 1000, 500).unwrap();
    
    // Transfer from A to B
    manager.update_balances("channel-1", 900, 600).unwrap();
    
    // Verify balances updated
    let channels = manager.list_open();
    let channel = channels.first().unwrap();
    assert_eq!(channel.balance_a, 900);
    assert_eq!(channel.balance_b, 600);
}

#[test]
fn test_multiple_channels() {
    let mut manager = ChannelManager::with_config(ChannelConfig::default());
    
    // Open multiple channels
    manager.open(test_did("alice"), test_did("bob"), 1000, 1000).unwrap();
    manager.open(test_did("alice"), test_did("charlie"), 500, 500).unwrap();
    manager.open(test_did("bob"), test_did("charlie"), 200, 200).unwrap();
    
    // Verify all channels
    let open_channels = manager.list_open();
    assert_eq!(open_channels.len(), 3);
    
    // Check stats
    let stats = manager.stats();
    assert_eq!(stats.open_channels, 3);
    assert_eq!(stats.total_value_locked, 3400);
}

#[test]
fn test_channel_close() {
    let mut manager = ChannelManager::new();
    
    let party_a = test_did("alice");
    let party_b = test_did("bob");
    
    // Open and then close
    manager.open(party_a.clone(), party_b.clone(), 1000, 500).unwrap();
    
    let closed = manager.close("channel-1").unwrap();
    assert_eq!(closed.balance_a, 1000);
    
    // Verify no open channels
    let open_channels = manager.list_open();
    assert!(open_channels.is_empty());
}

#[test]
fn test_channel_insufficient_balance() {
    let mut channel = Channel::new("test-channel", test_did("alice"), test_did("bob"), 100, 100);
    
    // Try to transfer more than balance
    let result = channel.transfer_a_to_b(200);
    assert!(result.is_err());
}

#[test]
fn test_channel_manager_stats() {
    let mut manager = ChannelManager::new();
    
    // Initial stats
    let stats = manager.stats();
    assert_eq!(stats.open_channels, 0);
    assert_eq!(stats.total_transactions, 0);
    
    // After opening channels
    manager.open(test_did("a"), test_did("b"), 100, 100).unwrap();
    manager.open(test_did("c"), test_did("d"), 200, 200).unwrap();
    
    let stats = manager.stats();
    assert_eq!(stats.open_channels, 2);
    assert_eq!(stats.total_value_locked, 600);
}