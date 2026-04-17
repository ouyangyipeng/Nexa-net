//! Micro-Receipt with Cryptographic Signatures and Hash Chain
//!
//! Implements the receipt protocol from ECONOMY_LAYER.md:
//! - Each receipt contains Ed25519 signatures from both payer and payee
//! - Receipts form a hash chain: each receipt references the previous one
//!   via `previous_receipt_hash`, making tampering detectable
//! - Receipt verification checks both signatures and hash chain integrity
//!
//! # Hash Chain
//!
//! The hash chain ensures that if any receipt is tampered with, all subsequent
//! receipts become invalid. This provides tamper evidence without requiring
//! on-chain storage of every receipt.
//!
//! ```text
//! receipt_1: hash(receipt_0) = genesis_hash
//! receipt_2: hash(receipt_1)
//! receipt_3: hash(receipt_2)
//! ...
//! ```
//!
//! Any modification to receipt_i invalidates the hash in receipt_{i+1},
//! cascading through the entire chain.

use crate::error::{Error, Result};
use crate::identity::KeyPair;
use crate::types::Did;
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Genesis hash for the first receipt in a chain (no predecessor)
pub const GENESIS_HASH: &str = "genesis";

/// Micro-receipt for a single micro-transaction
///
/// Each receipt is signed by both parties and linked to its predecessor
/// via a SHA-256 hash chain, providing tamper-evidence and non-repudiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicroReceipt {
    /// Unique receipt ID
    pub receipt_id: String,
    /// Call ID this receipt corresponds to
    pub call_id: String,
    /// Payer DID
    pub payer: String,
    /// Payee DID
    pub payee: String,
    /// Amount in micro-NEXA tokens
    pub amount: u64,
    /// Service endpoint that was called
    pub service_endpoint: String,
    /// Timestamp of receipt creation
    pub timestamp: DateTime<Utc>,
    /// SHA-256 hash of the previous receipt in the chain
    /// "genesis" for the first receipt
    pub previous_receipt_hash: String,
    /// Ed25519 signature from the payer
    pub payer_signature: Vec<u8>,
    /// Ed25519 signature from the payee (optional during async confirmation)
    pub payee_signature: Option<Vec<u8>>,
}

impl MicroReceipt {
    /// Create a new unsigned receipt
    ///
    /// The receipt must be signed by both parties before it is considered confirmed.
    /// Use `sign_payer()` and `sign_payee()` to add signatures.
    pub fn new(
        call_id: &str,
        payer: &Did,
        payee: &Did,
        amount: u64,
        service_endpoint: &str,
        previous_receipt_hash: &str,
    ) -> Self {
        Self {
            receipt_id: format!("receipt-{}", uuid::Uuid::new_v4()),
            call_id: call_id.to_string(),
            payer: payer.as_str().to_string(),
            payee: payee.as_str().to_string(),
            amount,
            service_endpoint: service_endpoint.to_string(),
            timestamp: Utc::now(),
            previous_receipt_hash: previous_receipt_hash.to_string(),
            payer_signature: Vec::new(),
            payee_signature: None,
        }
    }

    /// Create the first receipt in a hash chain (genesis receipt)
    pub fn new_genesis(
        call_id: &str,
        payer: &Did,
        payee: &Did,
        amount: u64,
        service_endpoint: &str,
    ) -> Self {
        Self::new(
            call_id,
            payer,
            payee,
            amount,
            service_endpoint,
            GENESIS_HASH,
        )
    }

    /// Compute the SHA-256 hash of this receipt for chain linking
    ///
    /// The hash covers all receipt fields EXCEPT signatures (payer_signature,
    /// payee_signature). This ensures the hash is deterministic before signing
    /// and can be computed by either party independently.
    pub fn compute_hash(&self) -> String {
        // Create a deterministic serialization of the receipt content
        // excluding signatures, so the hash is the same before and after signing
        let mut hasher = Sha256::new();
        hasher.update(self.receipt_id.as_bytes());
        hasher.update(self.call_id.as_bytes());
        hasher.update(self.payer.as_bytes());
        hasher.update(self.payee.as_bytes());
        hasher.update(self.amount.to_be_bytes());
        hasher.update(self.service_endpoint.as_bytes());
        hasher.update(self.timestamp.to_rfc3339().as_bytes());
        hasher.update(self.previous_receipt_hash.as_bytes());

        let hash: [u8; 32] = hasher.finalize().into();
        hex::encode(hash)
    }

    /// Get the message bytes that should be signed
    ///
    /// The signing message includes all receipt content plus the hash chain link.
    /// This ensures the signature covers the complete receipt context.
    fn signing_message(&self) -> Vec<u8> {
        // Pre-allocate: typical receipt signing message is ~200-300 bytes.
        // Avoids multiple reallocations during extend_from_slice calls.
        let estimated_size = self.receipt_id.len()
            + self.call_id.len()
            + self.payer.len()
            + self.payee.len()
            + 8  // amount u64
            + self.service_endpoint.len()
            + 30 // RFC3339 timestamp (~30 bytes)
            + self.previous_receipt_hash.len();
        let mut msg = Vec::with_capacity(estimated_size);
        msg.extend_from_slice(self.receipt_id.as_bytes());
        msg.extend_from_slice(self.call_id.as_bytes());
        msg.extend_from_slice(self.payer.as_bytes());
        msg.extend_from_slice(self.payee.as_bytes());
        msg.extend_from_slice(&self.amount.to_be_bytes());
        msg.extend_from_slice(self.service_endpoint.as_bytes());
        msg.extend_from_slice(self.timestamp.to_rfc3339().as_bytes());
        msg.extend_from_slice(self.previous_receipt_hash.as_bytes());
        msg
    }

    /// Sign the receipt as the payer using Ed25519
    ///
    /// The payer signs the receipt content to acknowledge the payment obligation.
    pub fn sign_payer(&mut self, payer_keypair: &KeyPair) -> Result<()> {
        let message = self.signing_message();
        let signature = payer_keypair.sign(&message)?;
        self.payer_signature = signature.to_bytes().to_vec();
        Ok(())
    }

    /// Sign the receipt as the payee using Ed25519
    ///
    /// The payee signs to confirm service delivery and accept the payment.
    pub fn sign_payee(&mut self, payee_keypair: &KeyPair) -> Result<()> {
        let message = self.signing_message();
        let signature = payee_keypair.sign(&message)?;
        self.payee_signature = Some(signature.to_bytes().to_vec());
        Ok(())
    }

    /// Check if the receipt has both payer and payee signatures
    pub fn is_confirmed(&self) -> bool {
        !self.payer_signature.is_empty() && self.payee_signature.is_some()
    }

    /// Check if the receipt has at least the payer signature
    pub fn is_payer_signed(&self) -> bool {
        !self.payer_signature.is_empty()
    }
}

/// Receipt verifier for checking signature validity and hash chain integrity
pub struct ReceiptVerifier;

impl ReceiptVerifier {
    /// Verify a receipt's payer signature against the payer's public key
    ///
    /// Returns Ok(true) if the signature is valid, Ok(false) if no signature present.
    pub fn verify_payer_signature(
        receipt: &MicroReceipt,
        payer_public_key: &VerifyingKey,
    ) -> Result<bool> {
        if receipt.payer_signature.is_empty() {
            return Ok(false);
        }

        let signature_bytes: [u8; 64] = receipt
            .payer_signature
            .as_slice()
            .try_into()
            .map_err(|_| Error::Internal("Invalid payer signature length".to_string()))?;

        let signature = Signature::from_bytes(&signature_bytes);
        let message = receipt.signing_message();

        match payer_public_key.verify(&message, &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false), // Signature mismatch - not an error, just invalid
        }
    }

    /// Verify a receipt's payee signature against the payee's public key
    ///
    /// Returns Ok(true) if the signature is valid, Ok(false) if no signature present.
    pub fn verify_payee_signature(
        receipt: &MicroReceipt,
        payee_public_key: &VerifyingKey,
    ) -> Result<bool> {
        let payee_sig = receipt.payee_signature.as_ref();
        if payee_sig.is_none() {
            return Ok(false);
        }

        let signature_bytes: [u8; 64] = payee_sig
            .unwrap()
            .as_slice()
            .try_into()
            .map_err(|_| Error::Internal("Invalid payee signature length".to_string()))?;

        let signature = Signature::from_bytes(&signature_bytes);
        let message = receipt.signing_message();

        match payee_public_key.verify(&message, &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false), // Signature mismatch - not an error, just invalid
        }
    }

    /// Verify both signatures on a confirmed receipt
    ///
    /// A fully confirmed receipt must have valid signatures from both parties.
    pub fn verify_both_signatures(
        receipt: &MicroReceipt,
        payer_public_key: &VerifyingKey,
        payee_public_key: &VerifyingKey,
    ) -> Result<bool> {
        let payer_valid = Self::verify_payer_signature(receipt, payer_public_key)?;
        let payee_valid = Self::verify_payee_signature(receipt, payee_public_key)?;
        Ok(payer_valid && payee_valid)
    }

    /// Verify a receipt's hash chain link against the previous receipt
    ///
    /// Checks that `receipt.previous_receipt_hash` matches the hash of the
    /// previous receipt. For genesis receipts, previous_receipt_hash should
    /// be "genesis".
    pub fn verify_hash_chain(
        receipt: &MicroReceipt,
        previous_receipt: Option<&MicroReceipt>,
    ) -> Result<bool> {
        if receipt.previous_receipt_hash == GENESIS_HASH {
            // Genesis receipt - no predecessor to verify
            return Ok(previous_receipt.is_none());
        }

        let prev = previous_receipt.ok_or_else(|| {
            Error::Internal("Previous receipt required for non-genesis receipt".to_string())
        })?;

        let expected_hash = prev.compute_hash();
        Ok(receipt.previous_receipt_hash == expected_hash)
    }

    /// Full receipt verification: signatures + hash chain integrity
    ///
    /// This is the complete verification flow:
    /// 1. Verify payer signature
    /// 2. Verify payee signature (if present)
    /// 3. Verify hash chain link
    /// 4. Check receipt is confirmed (both signatures present)
    pub fn verify_full(
        receipt: &MicroReceipt,
        payer_public_key: &VerifyingKey,
        payee_public_key: &VerifyingKey,
        previous_receipt: Option<&MicroReceipt>,
    ) -> Result<bool> {
        // Must be fully signed
        if !receipt.is_confirmed() {
            return Ok(false);
        }

        // Verify signatures
        if !Self::verify_both_signatures(receipt, payer_public_key, payee_public_key)? {
            return Ok(false);
        }

        // Verify hash chain
        Self::verify_hash_chain(receipt, previous_receipt)
    }
}

/// Receipt chain manager for tracking a sequence of receipts
///
/// Maintains the hash chain relationship between receipts and provides
/// efficient verification of chain integrity.
pub struct ReceiptChain {
    /// DID of the payer
    payer: Did,
    /// DID of the payee
    payee: Did,
    /// Ordered sequence of receipts
    receipts: Vec<MicroReceipt>,
}

impl ReceiptChain {
    /// Create a new receipt chain between two parties
    pub fn new(payer: Did, payee: Did) -> Self {
        Self {
            payer,
            payee,
            receipts: Vec::new(),
        }
    }

    /// Add a receipt to the chain
    ///
    /// The receipt's previous_receipt_hash must match the hash of the
    /// last receipt in the chain (or be "genesis" for the first receipt).
    pub fn add_receipt(&mut self, receipt: MicroReceipt) -> Result<()> {
        // Verify hash chain link
        let expected_hash = self
            .receipts
            .last()
            .map(|r| r.compute_hash())
            .unwrap_or(GENESIS_HASH.to_string());

        if receipt.previous_receipt_hash != expected_hash {
            return Err(Error::Internal(format!(
                "Hash chain mismatch: expected {}, got {}",
                expected_hash, receipt.previous_receipt_hash
            )));
        }

        self.receipts.push(receipt);
        Ok(())
    }

    /// Create and add a new receipt to the chain
    ///
    /// Convenience method that creates a receipt with the correct
    /// previous_receipt_hash and adds it to the chain.
    pub fn create_receipt(
        &mut self,
        call_id: &str,
        amount: u64,
        service_endpoint: &str,
    ) -> MicroReceipt {
        let prev_hash = self
            .receipts
            .last()
            .map(|r| r.compute_hash())
            .unwrap_or(GENESIS_HASH.to_string());

        MicroReceipt::new(
            call_id,
            &self.payer,
            &self.payee,
            amount,
            service_endpoint,
            &prev_hash,
        )
    }

    /// Verify the entire chain integrity
    ///
    /// Checks every hash chain link in the sequence.
    pub fn verify_chain_integrity(&self) -> Result<bool> {
        for i in 0..self.receipts.len() {
            let expected_hash = if i == 0 {
                GENESIS_HASH.to_string()
            } else {
                self.receipts[i - 1].compute_hash()
            };

            if self.receipts[i].previous_receipt_hash != expected_hash {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Get the total amount in the chain
    pub fn total_amount(&self) -> u64 {
        self.receipts.iter().map(|r| r.amount).sum()
    }

    /// Get the number of receipts in the chain
    pub fn len(&self) -> usize {
        self.receipts.len()
    }

    /// Check if the chain is empty
    pub fn is_empty(&self) -> bool {
        self.receipts.is_empty()
    }

    /// Get the last receipt in the chain
    pub fn last(&self) -> Option<&MicroReceipt> {
        self.receipts.last()
    }

    /// Get all receipts in the chain
    pub fn all_receipts(&self) -> &[MicroReceipt] {
        &self.receipts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::KeyPair;

    #[test]
    fn test_receipt_creation() {
        let payer = Did::new("did:nexa:alice");
        let payee = Did::new("did:nexa:bob");

        let receipt = MicroReceipt::new_genesis("call-123", &payer, &payee, 100, "/translate");

        assert_eq!(receipt.amount, 100);
        assert_eq!(receipt.previous_receipt_hash, GENESIS_HASH);
        assert!(!receipt.is_confirmed());
        assert!(!receipt.is_payer_signed());
    }

    #[test]
    fn test_receipt_signing_and_verification() {
        let payer = Did::new("did:nexa:alice");
        let payee = Did::new("did:nexa:bob");
        let payer_keypair = KeyPair::generate().unwrap();
        let payee_keypair = KeyPair::generate().unwrap();

        let mut receipt = MicroReceipt::new_genesis("call-123", &payer, &payee, 100, "/translate");

        // Sign as payer
        receipt.sign_payer(&payer_keypair).unwrap();
        assert!(receipt.is_payer_signed());
        assert!(!receipt.is_confirmed()); // No payee signature yet

        // Verify payer signature
        let payer_valid =
            ReceiptVerifier::verify_payer_signature(&receipt, payer_keypair.public_key().inner())
                .unwrap();
        assert!(payer_valid);

        // Sign as payee
        receipt.sign_payee(&payee_keypair).unwrap();
        assert!(receipt.is_confirmed());

        // Verify both signatures
        let both_valid = ReceiptVerifier::verify_both_signatures(
            &receipt,
            payer_keypair.public_key().inner(),
            payee_keypair.public_key().inner(),
        )
        .unwrap();
        assert!(both_valid);
    }

    #[test]
    fn test_receipt_wrong_key_fails_verification() {
        let payer = Did::new("did:nexa:alice");
        let payee = Did::new("did:nexa:bob");
        let payer_keypair = KeyPair::generate().unwrap();
        let wrong_keypair = KeyPair::generate().unwrap();
        let payee_keypair = KeyPair::generate().unwrap();

        let mut receipt = MicroReceipt::new_genesis("call-123", &payer, &payee, 100, "/translate");
        receipt.sign_payer(&payer_keypair).unwrap();
        receipt.sign_payee(&payee_keypair).unwrap();

        // Verify with wrong payer key should fail
        let wrong_payer =
            ReceiptVerifier::verify_payer_signature(&receipt, wrong_keypair.public_key().inner())
                .unwrap();
        assert!(!wrong_payer);

        // Verify with correct payer key should succeed
        let correct_payer =
            ReceiptVerifier::verify_payer_signature(&receipt, payer_keypair.public_key().inner())
                .unwrap();
        assert!(correct_payer);
    }

    #[test]
    fn test_receipt_hash_computation() {
        let payer = Did::new("did:nexa:alice");
        let payee = Did::new("did:nexa:bob");

        let receipt1 = MicroReceipt::new_genesis("call-1", &payer, &payee, 50, "/translate");
        let receipt2 = MicroReceipt::new_genesis("call-2", &payer, &payee, 75, "/translate");

        // Different receipts should produce different hashes
        assert_ne!(receipt1.compute_hash(), receipt2.compute_hash());

        // Same receipt should always produce the same hash
        assert_eq!(receipt1.compute_hash(), receipt1.compute_hash());
    }

    #[test]
    fn test_hash_chain_verification() {
        let payer = Did::new("did:nexa:alice");
        let payee = Did::new("did:nexa:bob");

        // Genesis receipt
        let receipt1 = MicroReceipt::new_genesis("call-1", &payer, &payee, 50, "/translate");

        // Verify genesis link (no previous receipt)
        let genesis_valid = ReceiptVerifier::verify_hash_chain(&receipt1, None).unwrap();
        assert!(genesis_valid);

        // Second receipt links to first
        let receipt2 = MicroReceipt::new(
            "call-2",
            &payer,
            &payee,
            75,
            "/translate",
            &receipt1.compute_hash(),
        );

        // Verify chain link
        let chain_valid = ReceiptVerifier::verify_hash_chain(&receipt2, Some(&receipt1)).unwrap();
        assert!(chain_valid);

        // Wrong previous receipt should fail
        let receipt3 = MicroReceipt::new_genesis("call-3", &payer, &payee, 25, "/translate");
        let wrong_chain = ReceiptVerifier::verify_hash_chain(&receipt2, Some(&receipt3)).unwrap();
        assert!(!wrong_chain);
    }

    #[test]
    fn test_receipt_chain_manager() {
        let payer = Did::new("did:nexa:alice");
        let payee = Did::new("did:nexa:bob");

        let mut chain = ReceiptChain::new(payer.clone(), payee.clone());
        assert!(chain.is_empty());

        // Add genesis receipt
        let receipt1 = MicroReceipt::new_genesis("call-1", &payer, &payee, 50, "/translate");
        chain.add_receipt(receipt1).unwrap();
        assert_eq!(chain.len(), 1);
        assert_eq!(chain.total_amount(), 50);

        // Add linked receipt
        let receipt2 = chain.create_receipt("call-2", 75, "/translate");
        chain.add_receipt(receipt2).unwrap();
        assert_eq!(chain.len(), 2);
        assert_eq!(chain.total_amount(), 125);

        // Verify chain integrity
        assert!(chain.verify_chain_integrity().unwrap());

        // Try to add receipt with wrong hash chain link
        let bad_receipt = MicroReceipt::new("call-3", &payer, &payee, 25, "/translate", "bad_hash");
        assert!(chain.add_receipt(bad_receipt).is_err());
    }

    #[test]
    fn test_receipt_json_roundtrip() {
        let payer = Did::new("did:nexa:alice");
        let payee = Did::new("did:nexa:bob");
        let payer_keypair = KeyPair::generate().unwrap();

        let mut receipt = MicroReceipt::new_genesis("call-123", &payer, &payee, 100, "/translate");
        receipt.sign_payer(&payer_keypair).unwrap();

        let json = serde_json::to_string(&receipt).unwrap();
        let deserialized: MicroReceipt = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.receipt_id, receipt.receipt_id);
        assert_eq!(deserialized.amount, receipt.amount);
        assert_eq!(deserialized.payer, receipt.payer);
        assert_eq!(deserialized.payer_signature, receipt.payer_signature);
    }

    #[test]
    fn test_full_receipt_verification_flow() {
        let payer = Did::new("did:nexa:alice");
        let payee = Did::new("did:nexa:bob");
        let payer_keypair = KeyPair::generate().unwrap();
        let payee_keypair = KeyPair::generate().unwrap();

        // Create and sign genesis receipt
        let mut receipt = MicroReceipt::new_genesis("call-1", &payer, &payee, 50, "/translate");
        receipt.sign_payer(&payer_keypair).unwrap();
        receipt.sign_payee(&payee_keypair).unwrap();

        // Full verification should pass
        let valid = ReceiptVerifier::verify_full(
            &receipt,
            payer_keypair.public_key().inner(),
            payee_keypair.public_key().inner(),
            None, // genesis receipt, no predecessor
        )
        .unwrap();
        assert!(valid);

        // Unconfirmed receipt should fail full verification
        let mut unsigned = MicroReceipt::new_genesis("call-2", &payer, &payee, 75, "/translate");
        let unsigned_valid = ReceiptVerifier::verify_full(
            &unsigned,
            payer_keypair.public_key().inner(),
            payee_keypair.public_key().inner(),
            None,
        )
        .unwrap();
        assert!(!unsigned_valid);
    }

    // ========== Boundary/Error Tests ==========

    #[test]
    fn test_receipt_zero_amount() {
        let payer = Did::new("did:nexa:alice");
        let payee = Did::new("did:nexa:bob");
        let payer_keypair = KeyPair::generate().unwrap();
        let payee_keypair = KeyPair::generate().unwrap();

        let mut receipt = MicroReceipt::new_genesis("call-1", &payer, &payee, 0, "/translate");
        receipt.sign_payer(&payer_keypair).unwrap();
        receipt.sign_payee(&payee_keypair).unwrap();

        assert!(receipt.is_confirmed());
        assert!(ReceiptVerifier::verify_both_signatures(
            &receipt,
            payer_keypair.public_key().inner(),
            payee_keypair.public_key().inner()
        )
        .unwrap());
    }

    #[test]
    fn test_receipt_verify_payer_empty_signature() {
        let payer = Did::new("did:nexa:alice");
        let payee = Did::new("did:nexa:bob");
        let payer_keypair = KeyPair::generate().unwrap();

        let receipt = MicroReceipt::new_genesis("call-1", &payer, &payee, 100, "/translate");
        // No payer signature — verify should return false
        let result =
            ReceiptVerifier::verify_payer_signature(&receipt, payer_keypair.public_key().inner())
                .unwrap();
        assert!(!result);
    }

    #[test]
    fn test_receipt_verify_payee_no_signature() {
        let payer = Did::new("did:nexa:alice");
        let payee = Did::new("did:nexa:bob");
        let payee_keypair = KeyPair::generate().unwrap();

        let receipt = MicroReceipt::new_genesis("call-1", &payer, &payee, 100, "/translate");
        // No payee signature — verify should return false
        let result =
            ReceiptVerifier::verify_payee_signature(&receipt, payee_keypair.public_key().inner())
                .unwrap();
        assert!(!result);
    }

    #[test]
    fn test_receipt_chain_empty_last() {
        let payer = Did::new("did:nexa:alice");
        let payee = Did::new("did:nexa:bob");

        let chain = ReceiptChain::new(payer, payee);
        assert!(chain.last().is_none());
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
        assert_eq!(chain.total_amount(), 0);
    }

    #[test]
    fn test_receipt_chain_integrity_empty() {
        let payer = Did::new("did:nexa:alice");
        let payee = Did::new("did:nexa:bob");

        let chain = ReceiptChain::new(payer, payee);
        assert!(chain.verify_chain_integrity().unwrap());
    }

    #[test]
    fn test_receipt_chain_multi_receipt_integrity() {
        let payer = Did::new("did:nexa:alice");
        let payee = Did::new("did:nexa:bob");

        let mut chain = ReceiptChain::new(payer.clone(), payee.clone());

        for i in 0..10 {
            let receipt = chain.create_receipt(&format!("call-{}", i), i * 10, "/translate");
            chain.add_receipt(receipt).unwrap();
        }

        assert_eq!(chain.len(), 10);
        assert!(chain.verify_chain_integrity().unwrap());
    }

    #[test]
    fn test_receipt_hash_is_deterministic_before_and_after_signing() {
        let payer = Did::new("did:nexa:alice");
        let payee = Did::new("did:nexa:bob");
        let payer_keypair = KeyPair::generate().unwrap();

        let mut receipt = MicroReceipt::new_genesis("call-1", &payer, &payee, 100, "/translate");
        let hash_before = receipt.compute_hash();

        receipt.sign_payer(&payer_keypair).unwrap();
        let hash_after = receipt.compute_hash();

        // Hash should be the same because signing doesn't change hash payload
        assert_eq!(hash_before, hash_after);
    }

    #[test]
    fn test_receipt_genesis_hash_chain_with_previous() {
        let payer = Did::new("did:nexa:alice");
        let payee = Did::new("did:nexa:bob");

        // Genesis receipt claims GENESIS_HASH but a previous receipt is provided
        let receipt = MicroReceipt::new_genesis("call-1", &payer, &payee, 50, "/translate");
        let fake_prev = MicroReceipt::new_genesis("call-0", &payer, &payee, 25, "/translate");
        let result = ReceiptVerifier::verify_hash_chain(&receipt, Some(&fake_prev)).unwrap();
        assert!(!result); // Genesis should have no previous
    }

    #[test]
    fn test_receipt_non_genesis_without_previous_fails() {
        let payer = Did::new("did:nexa:alice");
        let payee = Did::new("did:nexa:bob");

        // Non-genesis receipt without previous should fail
        let receipt = MicroReceipt::new("call-1", &payer, &payee, 50, "/translate", "some_hash");
        let result = ReceiptVerifier::verify_hash_chain(&receipt, None);
        assert!(result.is_err());
    }

    // ========== Proptest Tests ==========

    use proptest::prelude::*;

    proptest! {
        /// Receipt signature+hash chain verification round-trip
        #[test]
        fn proptest_receipt_sign_verify_roundtrip(
            amount in 0u64..10000,
            endpoint in "[a-zA-Z/]{3,20}",
        ) {
            let payer = Did::new("did:nexa:alice");
            let payee = Did::new("did:nexa:bob");
            let payer_keypair = KeyPair::generate().unwrap();
            let payee_keypair = KeyPair::generate().unwrap();

            let mut receipt = MicroReceipt::new_genesis("call-1", &payer, &payee, amount, &endpoint);
            receipt.sign_payer(&payer_keypair).unwrap();
            receipt.sign_payee(&payee_keypair).unwrap();

            // Full verification must pass
            let valid = ReceiptVerifier::verify_full(
                &receipt,
                payer_keypair.public_key().inner(),
                payee_keypair.public_key().inner(),
                None,
            ).unwrap();
            assert!(valid);

            // Wrong key must fail
            let wrong_keypair = KeyPair::generate().unwrap();
            let wrong_valid = ReceiptVerifier::verify_payer_signature(
                &receipt, wrong_keypair.public_key().inner()
            ).unwrap();
            assert!(!wrong_valid);
        }

        /// Receipt chain integrity holds for any sequence of receipts
        #[test]
        fn proptest_receipt_chain_integrity(
            amounts in prop::collection::vec(1u64..1000, 1..5),
        ) {
            let payer = Did::new("did:nexa:alice");
            let payee = Did::new("did:nexa:bob");

            let mut chain = ReceiptChain::new(payer.clone(), payee.clone());

            for (i, amount) in amounts.iter().enumerate() {
                let receipt = chain.create_receipt(&format!("call-{}", i), *amount, "/translate");
                chain.add_receipt(receipt).unwrap();
            }

            assert!(chain.verify_chain_integrity().unwrap());
            assert_eq!(chain.total_amount(), amounts.iter().sum::<u64>());
        }
    }
}
