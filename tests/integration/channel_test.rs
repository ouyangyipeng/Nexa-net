//! Channel Integration Tests
//!
//! Tests for state channel lifecycle and micro-transactions.
//! Updated to use common test utilities (TestEnvironment with ChannelConfig).

use super::common::TestEnvironment;
use nexa_net::economy::channel::ChannelConfig;
use nexa_net::economy::{Channel, ChannelManager};
use nexa_net::types::Did;
use std::time::Duration;

/// Create a test DID
fn test_did(name: &str) -> Did {
    Did::new(&format!("did:nexa:{}", name))
}

#[test]
fn test_channel_lifecycle() {
    let mut env = TestEnvironment::new();

    let party_a = test_did("alice");
    let party_b = test_did("bob");

    // Open channel — min_deposit=0 in test config, deposits must still be valid
    let channel = env
        .channel_manager
        .open(party_a.clone(), party_b.clone(), 1000, 500)
        .unwrap();
    assert_eq!(channel.balance_a, 1000);
    assert_eq!(channel.balance_b, 500);

    // Verify channel is listed
    let open_channels = env.channel_manager.list_open();
    assert_eq!(open_channels.len(), 1);
}

#[test]
fn test_channel_transfers() {
    let mut env = TestEnvironment::new();

    let party_a = test_did("alice");
    let party_b = test_did("bob");

    // Open channel
    let channel = env
        .channel_manager
        .open(party_a.clone(), party_b.clone(), 1000, 500)
        .unwrap();

    // Transfer from A to B: 1000→900, 500→600 (total preserved = 1500)
    env.channel_manager
        .update_balances(&channel.id, 900, 600)
        .unwrap();

    // Verify balances updated
    let channels = env.channel_manager.list_open();
    let updated = channels.first().unwrap();
    assert_eq!(updated.balance_a, 900);
    assert_eq!(updated.balance_b, 600);
}

#[test]
fn test_multiple_channels() {
    let mut env = TestEnvironment::new();

    // Open multiple channels
    env.channel_manager
        .open(test_did("alice"), test_did("bob"), 1000, 1000)
        .unwrap();
    env.channel_manager
        .open(test_did("alice"), test_did("charlie"), 500, 500)
        .unwrap();
    env.channel_manager
        .open(test_did("bob"), test_did("charlie"), 200, 200)
        .unwrap();

    // Verify all channels
    let open_channels = env.channel_manager.list_open();
    assert_eq!(open_channels.len(), 3);

    // Check stats
    let stats = env.channel_manager.stats();
    assert_eq!(stats.open_channels, 3);
    assert_eq!(stats.total_value_locked, 3400);
}

#[test]
fn test_channel_close_initiate() {
    let mut manager = ChannelManager::new();

    let party_a = test_did("alice");
    let party_b = test_did("bob");

    // Open channel
    manager
        .open(party_a.clone(), party_b.clone(), 1000, 500)
        .unwrap();

    // Initiate close (sets state to Closing)
    let channel = manager.get_mut("channel-1").unwrap();
    channel.initiate_close(Duration::from_secs(3600)).unwrap();

    // Verify channel is in closing state
    assert!(!channel.is_active());
    assert!(channel.is_closing());
}

#[test]
fn test_channel_insufficient_balance() {
    let channel = Channel::new("test-channel", test_did("alice"), test_did("bob"), 100, 100);

    // Try to transfer more than balance
    let mut channel = channel;
    let result = channel.transfer_a_to_b(200);
    assert!(result.is_err());
}

#[test]
fn test_channel_manager_stats() {
    let mut env = TestEnvironment::new();

    // Initial stats
    let stats = env.channel_manager.stats();
    assert_eq!(stats.open_channels, 0);
    assert_eq!(stats.total_transactions, 0);

    // After opening channels
    env.channel_manager
        .open(test_did("a"), test_did("b"), 100, 100)
        .unwrap();
    env.channel_manager
        .open(test_did("c"), test_did("d"), 200, 200)
        .unwrap();

    let stats = env.channel_manager.stats();
    assert_eq!(stats.open_channels, 2);
    assert_eq!(stats.total_value_locked, 600);
}

#[test]
fn test_channel_with_custom_config() {
    // Verify ChannelConfig with min_deposit=0 works
    let config = ChannelConfig {
        min_deposit: 0,
        max_deposit: 1_000_000,
        challenge_period: Duration::from_secs(3600),
        max_channels_per_peer: 10,
    };
    let mut manager = ChannelManager::with_config(config);

    // Can open with zero deposit
    let channel = manager
        .open(test_did("zero-a"), test_did("zero-b"), 0, 0)
        .unwrap();
    assert_eq!(channel.balance_a, 0);
    assert_eq!(channel.balance_b, 0);
}
