//! State Channel Management
//!
//! Layer 2 payment channels for high-frequency micro-transactions.
//!
//! # Channel Lifecycle
//!
//! ```text
//! Created -> Open -> (Active) -> Closing -> Closed
//!                \-> Disputed -> Challenged -> Settled
//! ```

use crate::error::{Error, Result};
use crate::types::{ChannelState, Did};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::time::Duration;

/// Channel ID
pub type ChannelId = String;

/// Channel configuration
#[derive(Debug, Clone)]
pub struct ChannelConfig {
    /// Minimum deposit required
    pub min_deposit: u64,
    /// Maximum deposit allowed
    pub max_deposit: u64,
    /// Challenge period duration
    pub challenge_period: Duration,
    /// Maximum open channels per peer
    pub max_channels_per_peer: usize,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            min_deposit: 10,
            max_deposit: 1_000_000,
            challenge_period: Duration::from_secs(3600), // 1 hour
            max_channels_per_peer: 10,
        }
    }
}

/// Channel dispute state
#[derive(Debug, Clone)]
pub struct DisputeState {
    /// Dispute initiator
    pub initiator: String,
    /// Dispute reason
    pub reason: String,
    /// Dispute timestamp
    pub timestamp: DateTime<Utc>,
    /// Challenge end time
    pub challenge_end: DateTime<Utc>,
    /// Proposed balance A
    pub proposed_balance_a: u64,
    /// Proposed balance B
    pub proposed_balance_b: u64,
    /// Evidence (receipts, signatures)
    pub evidence: Vec<Vec<u8>>,
}

/// State channel
#[derive(Debug, Clone)]
pub struct Channel {
    /// Channel ID
    pub id: ChannelId,
    /// Party A DID (initiator)
    pub party_a: Did,
    /// Party B DID (counterparty)
    pub party_b: Did,
    /// Party A balance
    pub balance_a: u64,
    /// Party B balance
    pub balance_b: u64,
    /// Initial deposit A
    pub deposit_a: u64,
    /// Initial deposit B
    pub deposit_b: u64,
    /// Channel state
    pub state: ChannelState,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Updated at
    pub updated_at: DateTime<Utc>,
    /// Sequence number (for state updates)
    pub sequence: u64,
    /// Dispute state (if any)
    pub dispute: Option<DisputeState>,
    /// Settlement deadline (for closing channels)
    pub settlement_deadline: Option<DateTime<Utc>>,
    /// Total transactions processed
    pub total_transactions: u64,
    /// Total value transferred
    pub total_transferred: u64,
}

impl Channel {
    /// Create a new channel
    pub fn new(id: &str, party_a: Did, party_b: Did, deposit_a: u64, deposit_b: u64) -> Self {
        let now = Utc::now();
        Self {
            id: id.to_string(),
            party_a,
            party_b,
            balance_a: deposit_a,
            balance_b: deposit_b,
            deposit_a,
            deposit_b,
            state: ChannelState::Open,
            created_at: now,
            updated_at: now,
            sequence: 0,
            dispute: None,
            settlement_deadline: None,
            total_transactions: 0,
            total_transferred: 0,
        }
    }

    /// Get total balance (should remain constant)
    pub fn total_balance(&self) -> u64 {
        self.balance_a + self.balance_b
    }

    /// Get total deposit
    pub fn total_deposit(&self) -> u64 {
        self.deposit_a + self.deposit_b
    }

    /// Check if channel is active
    pub fn is_active(&self) -> bool {
        matches!(self.state, ChannelState::Open)
    }

    /// Check if channel is closing
    pub fn is_closing(&self) -> bool {
        matches!(self.state, ChannelState::Closing)
    }

    /// Check if channel is closed
    pub fn is_closed(&self) -> bool {
        matches!(self.state, ChannelState::Closed)
    }

    /// Update balances (off-chain)
    pub fn update(&mut self, new_balance_a: u64, new_balance_b: u64) -> Result<()> {
        if !self.is_active() {
            return Err(Error::ChannelOperation("Channel not open".to_string()));
        }

        if new_balance_a + new_balance_b != self.total_balance() {
            return Err(Error::ChannelOperation("Balance mismatch".to_string()));
        }

        // Track transfer direction
        let transfer = if new_balance_a < self.balance_a {
            self.balance_a - new_balance_a // A paid B
        } else if new_balance_b < self.balance_b {
            self.balance_b - new_balance_b // B paid A
        } else {
            0
        };

        self.balance_a = new_balance_a;
        self.balance_b = new_balance_b;
        self.sequence += 1;
        self.total_transactions += 1;
        self.total_transferred += transfer;
        self.updated_at = Utc::now();

        Ok(())
    }

    /// Transfer from A to B
    pub fn transfer_a_to_b(&mut self, amount: u64) -> Result<()> {
        if amount > self.balance_a {
            return Err(Error::ChannelOperation("Insufficient balance".to_string()));
        }
        self.update(self.balance_a - amount, self.balance_b + amount)
    }

    /// Transfer from B to A
    pub fn transfer_b_to_a(&mut self, amount: u64) -> Result<()> {
        if amount > self.balance_b {
            return Err(Error::ChannelOperation("Insufficient balance".to_string()));
        }
        self.update(self.balance_a + amount, self.balance_b - amount)
    }

    /// Initiate closing
    pub fn initiate_close(&mut self, challenge_period: Duration) -> Result<()> {
        if !self.is_active() {
            return Err(Error::ChannelOperation("Channel not open".to_string()));
        }
        self.state = ChannelState::Closing;
        self.settlement_deadline =
            Some(Utc::now() + chrono::Duration::from_std(challenge_period).unwrap());
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Finalize closing
    pub fn finalize_close(&mut self) -> Result<()> {
        if !self.is_closing() {
            return Err(Error::ChannelOperation("Channel not closing".to_string()));
        }

        // Check if challenge period has passed
        if let Some(deadline) = self.settlement_deadline {
            if Utc::now() < deadline {
                return Err(Error::ChannelOperation(
                    "Challenge period not over".to_string(),
                ));
            }
        }

        self.state = ChannelState::Closed;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Raise dispute
    pub fn raise_dispute(
        &mut self,
        initiator: &str,
        reason: &str,
        challenge_period: Duration,
    ) -> Result<()> {
        if !self.is_active() && !self.is_closing() {
            return Err(Error::ChannelOperation(
                "Cannot dispute in current state".to_string(),
            ));
        }

        let now = Utc::now();
        self.dispute = Some(DisputeState {
            initiator: initiator.to_string(),
            reason: reason.to_string(),
            timestamp: now,
            challenge_end: now + chrono::Duration::from_std(challenge_period).unwrap(),
            proposed_balance_a: self.balance_a,
            proposed_balance_b: self.balance_b,
            evidence: Vec::new(),
        });
        self.state = ChannelState::Disputed;
        self.updated_at = now;
        Ok(())
    }

    /// Add evidence to dispute
    pub fn add_evidence(&mut self, evidence: Vec<u8>) -> Result<()> {
        if let Some(ref mut dispute) = self.dispute {
            dispute.evidence.push(evidence);
            self.updated_at = Utc::now();
            Ok(())
        } else {
            Err(Error::ChannelOperation("No active dispute".to_string()))
        }
    }

    /// Resolve dispute
    pub fn resolve_dispute(&mut self, final_balance_a: u64, final_balance_b: u64) -> Result<()> {
        if self.dispute.is_none() {
            return Err(Error::ChannelOperation("No active dispute".to_string()));
        }

        if final_balance_a + final_balance_b != self.total_balance() {
            return Err(Error::ChannelOperation("Balance mismatch".to_string()));
        }

        self.balance_a = final_balance_a;
        self.balance_b = final_balance_b;
        self.dispute = None;
        self.state = ChannelState::Closed;
        self.sequence += 1;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Get channel age
    pub fn age(&self) -> Duration {
        (Utc::now() - self.created_at)
            .to_std()
            .unwrap_or(Duration::ZERO)
    }

    /// Get time since last update
    pub fn idle_time(&self) -> Duration {
        (Utc::now() - self.updated_at)
            .to_std()
            .unwrap_or(Duration::ZERO)
    }
}

/// Channel manager
#[derive(Debug)]
pub struct ChannelManager {
    /// Active channels
    channels: HashMap<ChannelId, Channel>,
    /// Channel configuration
    config: ChannelConfig,
    /// Channel counter
    channel_counter: u64,
}

impl ChannelManager {
    /// Create a new channel manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a channel manager with custom config
    pub fn with_config(config: ChannelConfig) -> Self {
        Self {
            channels: HashMap::new(),
            config,
            channel_counter: 0,
        }
    }

    /// Generate a new channel ID
    fn generate_channel_id(&mut self) -> ChannelId {
        self.channel_counter += 1;
        format!("channel-{}", self.channel_counter)
    }

    /// Open a new channel
    pub fn open(
        &mut self,
        party_a: Did,
        party_b: Did,
        deposit_a: u64,
        deposit_b: u64,
    ) -> Result<Channel> {
        // Validate deposits
        if deposit_a < self.config.min_deposit || deposit_b < self.config.min_deposit {
            return Err(Error::ChannelOperation("Deposit below minimum".to_string()));
        }

        if deposit_a > self.config.max_deposit || deposit_b > self.config.max_deposit {
            return Err(Error::ChannelOperation("Deposit above maximum".to_string()));
        }

        // Check channel limit per peer
        let peer_channels = self
            .channels
            .values()
            .filter(|c| {
                c.party_a.as_str() == party_a.as_str() || c.party_b.as_str() == party_b.as_str()
            })
            .count();

        if peer_channels >= self.config.max_channels_per_peer {
            return Err(Error::ChannelOperation(
                "Maximum channels reached".to_string(),
            ));
        }

        let id = self.generate_channel_id();
        let channel = Channel::new(&id, party_a, party_b, deposit_a, deposit_b);
        self.channels.insert(id.clone(), channel.clone());

        Ok(channel)
    }

    /// Get a channel by ID
    pub fn get(&self, id: &str) -> Option<&Channel> {
        self.channels.get(id)
    }

    /// Get a mutable channel by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Channel> {
        self.channels.get_mut(id)
    }

    /// Update channel balances
    pub fn update_balances(
        &mut self,
        channel_id: &str,
        balance_a: u64,
        balance_b: u64,
    ) -> Result<()> {
        let channel = self
            .channels
            .get_mut(channel_id)
            .ok_or_else(|| Error::ChannelOperation("Channel not found".to_string()))?;
        channel.update(balance_a, balance_b)
    }

    /// Close a channel
    pub fn close(&mut self, channel_id: &str) -> Result<Channel> {
        let channel = self
            .channels
            .get_mut(channel_id)
            .ok_or_else(|| Error::ChannelOperation("Channel not found".to_string()))?;

        if channel.is_active() {
            channel.initiate_close(self.config.challenge_period)?;
        }

        channel.finalize_close()?;

        Ok(channel.clone())
    }

    /// Force close a channel (with dispute)
    pub fn force_close(&mut self, channel_id: &str, reason: &str) -> Result<Channel> {
        let channel = self
            .channels
            .get_mut(channel_id)
            .ok_or_else(|| Error::ChannelOperation("Channel not found".to_string()))?;

        let initiator = channel.party_a.as_str().to_string();
        channel.raise_dispute(&initiator, reason, self.config.challenge_period)?;

        Ok(channel.clone())
    }

    /// List all channels
    pub fn list_all(&self) -> Vec<&Channel> {
        self.channels.values().collect()
    }

    /// List open channels
    pub fn list_open(&self) -> Vec<&Channel> {
        self.channels.values().filter(|c| c.is_active()).collect()
    }

    /// List channels for a peer
    pub fn list_for_peer(&self, did: &Did) -> Vec<&Channel> {
        self.channels
            .values()
            .filter(|c| c.party_a.as_str() == did.as_str() || c.party_b.as_str() == did.as_str())
            .collect()
    }

    /// Remove closed channels
    pub fn cleanup_closed(&mut self) -> Vec<ChannelId> {
        let closed: Vec<ChannelId> = self
            .channels
            .iter()
            .filter(|(_, c)| c.is_closed())
            .map(|(id, _)| id.clone())
            .collect();

        for id in &closed {
            self.channels.remove(id);
        }

        closed
    }

    /// Get statistics
    pub fn stats(&self) -> ChannelManagerStats {
        let total = self.channels.len();
        let open = self.channels.values().filter(|c| c.is_active()).count();
        let closing = self.channels.values().filter(|c| c.is_closing()).count();
        let closed = self.channels.values().filter(|c| c.is_closed()).count();

        let total_value = self.channels.values().map(|c| c.total_balance()).sum();

        let total_transactions = self.channels.values().map(|c| c.total_transactions).sum();

        ChannelManagerStats {
            total_channels: total,
            open_channels: open,
            closing_channels: closing,
            closed_channels: closed,
            total_value_locked: total_value,
            total_transactions,
        }
    }
}

impl Default for ChannelManager {
    fn default() -> Self {
        Self {
            channels: HashMap::new(),
            config: ChannelConfig::default(),
            channel_counter: 0,
        }
    }
}

/// Channel manager statistics
#[derive(Debug, Clone)]
pub struct ChannelManagerStats {
    /// Total channels
    pub total_channels: usize,
    /// Open channels
    pub open_channels: usize,
    /// Closing channels
    pub closing_channels: usize,
    /// Closed channels
    pub closed_channels: usize,
    /// Total value locked in channels
    pub total_value_locked: u64,
    /// Total transactions processed
    pub total_transactions: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_did(s: &str) -> Did {
        Did::new(s)
    }

    #[test]
    fn test_channel_creation() {
        let party_a = test_did("did:nexa:alice");
        let party_b = test_did("did:nexa:bob");

        let channel = Channel::new("ch-1", party_a.clone(), party_b.clone(), 100, 50);

        assert_eq!(channel.total_balance(), 150);
        assert!(channel.is_active());
    }

    #[test]
    fn test_channel_transfer() {
        let party_a = test_did("did:nexa:alice");
        let party_b = test_did("did:nexa:bob");

        let mut channel = Channel::new("ch-1", party_a, party_b, 100, 50);

        channel.transfer_a_to_b(30).unwrap();

        assert_eq!(channel.balance_a, 70);
        assert_eq!(channel.balance_b, 80);
        assert_eq!(channel.total_transactions, 1);
        assert_eq!(channel.total_transferred, 30);
    }

    #[test]
    fn test_channel_insufficient_balance() {
        let party_a = test_did("did:nexa:alice");
        let party_b = test_did("did:nexa:bob");

        let mut channel = Channel::new("ch-1", party_a, party_b, 100, 50);

        let result = channel.transfer_a_to_b(200);
        assert!(result.is_err());
    }

    #[test]
    fn test_channel_manager() {
        let mut manager = ChannelManager::new();

        let party_a = test_did("did:nexa:alice");
        let party_b = test_did("did:nexa:bob");

        let channel = manager.open(party_a, party_b, 100, 50).unwrap();

        assert!(manager.get(&channel.id).is_some());
        assert_eq!(manager.list_open().len(), 1);
    }

    #[test]
    fn test_channel_close() {
        let mut manager = ChannelManager::new();

        let party_a = test_did("did:nexa:alice");
        let party_b = test_did("did:nexa:bob");

        let channel = manager.open(party_a, party_b, 100, 50).unwrap();
        let channel_id = channel.id.clone();

        // Initiate close
        manager
            .get_mut(&channel_id)
            .unwrap()
            .initiate_close(Duration::from_secs(0))
            .unwrap();

        // Finalize (would need to wait for challenge period in real scenario)
        // For test, we manually set the deadline to past
        {
            let ch = manager.get_mut(&channel_id).unwrap();
            ch.settlement_deadline = Some(Utc::now() - chrono::Duration::seconds(1));
        }

        manager
            .get_mut(&channel_id)
            .unwrap()
            .finalize_close()
            .unwrap();

        let closed = manager.get(&channel_id).unwrap();
        assert!(closed.is_closed());
    }

    #[test]
    fn test_channel_stats() {
        let mut manager = ChannelManager::new();

        let party_a = test_did("did:nexa:alice");
        let party_b = test_did("did:nexa:bob");

        manager
            .open(party_a.clone(), party_b.clone(), 100, 50)
            .unwrap();
        manager.open(party_a, party_b, 200, 100).unwrap();

        let stats = manager.stats();
        assert_eq!(stats.open_channels, 2);
        assert_eq!(stats.total_value_locked, 450);
    }
}
