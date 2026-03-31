//! Micro-Receipt
//!
//! Receipts for individual micro-transactions.

use crate::error::Result;
use crate::types::Did;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Micro-receipt for a single transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicroReceipt {
    /// Receipt ID
    pub receipt_id: String,
    /// Call ID
    pub call_id: String,
    /// Payer DID
    pub payer: String,
    /// Payee DID
    pub payee: String,
    /// Amount in micro-NEXA
    pub amount: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Payer signature
    pub payer_signature: Vec<u8>,
    /// Payee signature (optional)
    pub payee_signature: Option<Vec<u8>>,
}

impl MicroReceipt {
    /// Create a new receipt
    pub fn new(call_id: &str, payer: Did, payee: Did, amount: u64) -> Self {
        Self {
            receipt_id: format!("receipt-{}", uuid::Uuid::new_v4()),
            call_id: call_id.to_string(),
            payer: payer.as_str().to_string(),
            payee: payee.as_str().to_string(),
            amount,
            timestamp: Utc::now(),
            payer_signature: vec![],
            payee_signature: None,
        }
    }

    /// Sign the receipt as payer
    pub fn sign_payer(&mut self, signature: Vec<u8>) {
        self.payer_signature = signature;
    }

    /// Sign the receipt as payee
    pub fn sign_payee(&mut self, signature: Vec<u8>) {
        self.payee_signature = Some(signature);
    }

    /// Check if receipt is fully signed
    pub fn is_confirmed(&self) -> bool {
        !self.payer_signature.is_empty() && self.payee_signature.is_some()
    }
}

/// Receipt signer
pub struct ReceiptSigner;

/// Receipt verifier
pub struct ReceiptVerifier;

impl ReceiptVerifier {
    /// Verify a receipt
    pub fn verify(_receipt: &MicroReceipt) -> Result<bool> {
        // TODO: Implement actual signature verification
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receipt_creation() {
        let payer = Did::new("did:nexa:alice");
        let payee = Did::new("did:nexa:bob");

        let receipt = MicroReceipt::new("call-123", payer, payee, 100);

        assert_eq!(receipt.amount, 100);
        assert!(!receipt.is_confirmed());
    }

    #[test]
    fn test_receipt_signing() {
        let payer = Did::new("did:nexa:alice");
        let payee = Did::new("did:nexa:bob");

        let mut receipt = MicroReceipt::new("call-123", payer, payee, 100);
        receipt.sign_payer(vec![1, 2, 3]);
        receipt.sign_payee(vec![4, 5, 6]);

        assert!(receipt.is_confirmed());
    }
}
