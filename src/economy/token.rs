//! Token Engine
//!
//! Nexa-Token management and balance tracking.

use crate::error::{Error, Result};
use std::collections::HashMap;

/// Token amount (in micro-NEXA)
pub type TokenAmount = u64;

/// Token balance
#[derive(Debug, Clone, Default)]
pub struct TokenBalance {
    /// Available balance
    pub available: TokenAmount,
    /// Locked in channels
    pub locked: TokenAmount,
    /// Pending settlement
    pub pending: TokenAmount,
}

impl TokenBalance {
    /// Create a new balance
    pub fn new(available: TokenAmount) -> Self {
        Self {
            available,
            locked: 0,
            pending: 0,
        }
    }

    /// Get total balance
    pub fn total(&self) -> TokenAmount {
        self.available + self.locked + self.pending
    }
}

/// Token engine for managing tokens
#[derive(Debug, Clone, Default)]
pub struct TokenEngine {
    /// Balances by DID
    balances: HashMap<String, TokenBalance>,
}

impl TokenEngine {
    /// Create a new token engine
    pub fn new() -> Self {
        Self::default()
    }

    /// Get balance for a DID
    pub fn get_balance(&self, did: &str) -> TokenBalance {
        self.balances.get(did).cloned().unwrap_or_default()
    }

    /// Mint tokens to a DID
    pub fn mint(&mut self, did: &str, amount: TokenAmount) -> Result<()> {
        let balance = self.balances.entry(did.to_string()).or_default();
        balance.available += amount;
        Ok(())
    }

    /// Transfer tokens between DIDs
    pub fn transfer(&mut self, from: &str, to: &str, amount: TokenAmount) -> Result<()> {
        let from_balance = self
            .balances
            .get_mut(from)
            .ok_or_else(|| Error::InsufficientBalance(amount, 0))?;

        if from_balance.available < amount {
            return Err(Error::InsufficientBalance(amount, from_balance.available));
        }

        from_balance.available -= amount;

        let to_balance = self.balances.entry(to.to_string()).or_default();
        to_balance.available += amount;

        Ok(())
    }

    /// Lock tokens for a channel
    pub fn lock(&mut self, did: &str, amount: TokenAmount) -> Result<()> {
        let balance = self
            .balances
            .get_mut(did)
            .ok_or_else(|| Error::InsufficientBalance(amount, 0))?;

        if balance.available < amount {
            return Err(Error::InsufficientBalance(amount, balance.available));
        }

        balance.available -= amount;
        balance.locked += amount;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_balance() {
        let balance = TokenBalance::new(1000);
        assert_eq!(balance.available, 1000);
        assert_eq!(balance.total(), 1000);
    }

    #[test]
    fn test_token_engine() {
        let mut engine = TokenEngine::new();

        engine.mint("did:nexa:test", 1000).unwrap();
        assert_eq!(engine.get_balance("did:nexa:test").available, 1000);

        engine
            .transfer("did:nexa:test", "did:nexa:other", 500)
            .unwrap();
        assert_eq!(engine.get_balance("did:nexa:test").available, 500);
        assert_eq!(engine.get_balance("did:nexa:other").available, 500);
    }
}
