//! Settlement Engine
//!
//! Settlement and dispute resolution.

use crate::error::Result;
use crate::economy::{Channel, ChannelId, MicroReceipt};
use chrono::{DateTime, Utc};

/// Settlement record
#[derive(Debug, Clone)]
pub struct Settlement {
    /// Settlement ID
    pub id: String,
    /// Channel ID
    pub channel_id: ChannelId,
    /// Final balance for party A
    pub balance_a: u64,
    /// Final balance for party B
    pub balance_b: u64,
    /// Settlement timestamp
    pub timestamp: DateTime<Utc>,
    /// Settlement status
    pub status: SettlementStatus,
}

/// Settlement status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettlementStatus {
    /// Pending
    Pending,
    /// Confirmed
    Confirmed,
    /// Disputed
    Disputed,
    /// Finalized
    Finalized,
}

/// Dispute record
#[derive(Debug, Clone)]
pub struct Dispute {
    /// Dispute ID
    pub id: String,
    /// Channel ID
    pub channel_id: ChannelId,
    /// Disputed receipts
    pub receipts: Vec<MicroReceipt>,
    /// Dispute reason
    pub reason: String,
    /// Created at
    pub created_at: DateTime<Utc>,
}

/// Settlement engine
#[derive(Debug, Clone, Default)]
pub struct SettlementEngine {
    /// Pending settlements
    settlements: Vec<Settlement>,
}

impl SettlementEngine {
    /// Create a new settlement engine
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a settlement from a channel
    pub fn create_settlement(&mut self, channel: &Channel) -> Result<Settlement> {
        let settlement = Settlement {
            id: format!("settlement-{}", uuid::Uuid::new_v4()),
            channel_id: channel.id.clone(),
            balance_a: channel.balance_a,
            balance_b: channel.balance_b,
            timestamp: Utc::now(),
            status: SettlementStatus::Pending,
        };
        
        self.settlements.push(settlement.clone());
        Ok(settlement)
    }
    
    /// Finalize a settlement
    pub fn finalize(&mut self, settlement_id: &str) -> Result<Settlement> {
        let settlement = self.settlements.iter_mut()
            .find(|s| s.id == settlement_id)
            .ok_or_else(|| crate::error::Error::Settlement("Settlement not found".to_string()))?;
        
        settlement.status = SettlementStatus::Finalized;
        Ok(settlement.clone())
    }
    
    /// Create a dispute
    pub fn create_dispute(&self, channel_id: &str, reason: &str) -> Dispute {
        Dispute {
            id: format!("dispute-{}", uuid::Uuid::new_v4()),
            channel_id: channel_id.to_string(),
            receipts: vec![],
            reason: reason.to_string(),
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Did;

    #[test]
    fn test_settlement_creation() {
        let mut engine = SettlementEngine::new();
        
        let party_a = Did::new("did:nexa:alice");
        let party_b = Did::new("did:nexa:bob");
        let channel = Channel::new("channel-1", party_a, party_b, 1000, 500);
        
        let settlement = engine.create_settlement(&channel).unwrap();
        
        assert_eq!(settlement.balance_a, 1000);
        assert_eq!(settlement.status, SettlementStatus::Pending);
    }
}