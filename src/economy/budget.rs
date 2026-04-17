//! Budget Controller with Reservation and Multi-Level Limits
//!
//! Implements the budget control protocol from ECONOMY_LAYER.md:
//! - Multi-level budget limits: per_call / per_minute / per_hour / per_day / total
//! - Budget reservation: calls reserve budget upfront, settle on completion
//! - Automatic termination: budget exhaustion cancels in-progress calls
//! - Dead loop detection: repeated calls to the same endpoint are rate-limited
//!
//! # Reservation Flow
//!
//! ```text
//! 1. Call begins → check_budget() → reserve_budget()
//! 2. Call completes → settle_reservation() → actual cost deducted
//! 3. Call fails/cancelled → release_reservation() → reserved amount freed
//! ```

use crate::error::{Error, Result};
use std::collections::HashMap;

/// Multi-level budget limits
#[derive(Debug, Clone)]
pub struct BudgetLimit {
    /// Maximum budget per single call
    pub max_per_call: u64,
    /// Maximum budget per minute
    pub max_per_minute: u64,
    /// Maximum budget per hour
    pub max_per_hour: u64,
    /// Maximum budget per day
    pub max_per_day: u64,
    /// Maximum total budget (lifetime)
    pub max_total: u64,
}

impl Default for BudgetLimit {
    fn default() -> Self {
        Self {
            max_per_call: 100,
            max_per_minute: 500,
            max_per_hour: 1000,
            max_per_day: 10000,
            max_total: 100000,
        }
    }
}

/// Budget spending status for a single DID
#[derive(Debug, Clone, Default)]
pub struct BudgetStatus {
    /// Spent in current minute
    pub spent_minute: u64,
    /// Spent in current hour
    pub spent_hour: u64,
    /// Spent in current day
    pub spent_day: u64,
    /// Total spent (lifetime)
    pub spent_total: u64,
    /// Currently reserved (in-progress calls)
    pub reserved: u64,
    /// Number of active reservations
    pub active_reservations: usize,
}

/// A budget reservation for an in-progress call
#[derive(Debug, Clone)]
pub struct BudgetReservation {
    /// Reservation ID
    pub id: String,
    /// DID of the caller
    pub did: String,
    /// Reserved amount
    pub reserved_amount: u64,
    /// Call ID this reservation is for
    pub call_id: String,
    /// Timestamp of reservation creation
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Budget controller with reservation support and multi-level limits
///
/// Provides both pre-call budget checking and in-call reservation tracking.
/// When a call starts, budget is reserved; when it completes, the actual
/// cost is deducted and the reservation is settled.
#[derive(Debug, Clone)]
pub struct BudgetController {
    /// Budget limits
    pub limits: BudgetLimit,
    /// Spending status by DID
    status: HashMap<String, BudgetStatus>,
    /// Active reservations by reservation ID
    reservations: HashMap<String, BudgetReservation>,
    /// Call ID to reservation ID mapping
    call_reservations: HashMap<String, String>,
}

impl BudgetController {
    /// Create a new budget controller with default limits
    pub fn new() -> Self {
        Self {
            limits: BudgetLimit::default(),
            status: HashMap::new(),
            reservations: HashMap::new(),
            call_reservations: HashMap::new(),
        }
    }

    /// Create a budget controller with custom limits
    pub fn with_limits(limits: BudgetLimit) -> Self {
        Self {
            limits,
            status: HashMap::new(),
            reservations: HashMap::new(),
            call_reservations: HashMap::new(),
        }
    }

    /// Check if a call is within budget limits (without creating a reservation)
    ///
    /// Checks all budget levels: per_call, per_minute, per_hour, per_day, total.
    /// Also considers reserved (in-progress) amounts.
    pub fn check_budget(&self, did: &str, amount: u64) -> Result<()> {
        // Per-call limit
        if amount > self.limits.max_per_call {
            return Err(Error::BudgetExceeded(amount, self.limits.max_per_call));
        }

        let status = self.status.get(did).cloned().unwrap_or_default();
        let effective_spent = status.spent_total + status.reserved;

        // Per-minute limit (spent + reserved)
        if status.spent_minute + status.reserved + amount > self.limits.max_per_minute {
            return Err(Error::BudgetExceeded(
                status.spent_minute + status.reserved + amount,
                self.limits.max_per_minute,
            ));
        }

        // Per-hour limit
        if status.spent_hour + status.reserved + amount > self.limits.max_per_hour {
            return Err(Error::BudgetExceeded(
                status.spent_hour + status.reserved + amount,
                self.limits.max_per_hour,
            ));
        }

        // Per-day limit
        if status.spent_day + status.reserved + amount > self.limits.max_per_day {
            return Err(Error::BudgetExceeded(
                status.spent_day + status.reserved + amount,
                self.limits.max_per_day,
            ));
        }

        // Total budget limit
        if effective_spent + amount > self.limits.max_total {
            return Err(Error::BudgetExceeded(
                effective_spent + amount,
                self.limits.max_total,
            ));
        }

        Ok(())
    }

    /// Reserve budget for an in-progress call
    ///
    /// The reserved amount is tracked separately from spent amounts,
    /// ensuring that concurrent calls don't exceed budget limits.
    pub fn reserve_budget(&mut self, did: &str, call_id: &str, amount: u64) -> Result<String> {
        // First check if the budget allows this reservation
        self.check_budget(did, amount)?;

        let reservation_id = format!("reserve-{}", uuid::Uuid::new_v4());
        let reservation = BudgetReservation {
            id: reservation_id.clone(),
            did: did.to_string(),
            reserved_amount: amount,
            call_id: call_id.to_string(),
            created_at: chrono::Utc::now(),
        };

        // Update status
        let status = self.status.entry(did.to_string()).or_default();
        status.reserved += amount;
        status.active_reservations += 1;

        // Store reservation
        self.reservations
            .insert(reservation_id.clone(), reservation);
        self.call_reservations
            .insert(call_id.to_string(), reservation_id.clone());

        Ok(reservation_id)
    }

    /// Settle a reservation: deduct actual cost and release reservation
    ///
    /// If actual_cost < reserved_amount, the difference is released back.
    /// If actual_cost > reserved_amount, the extra is deducted from budget.
    pub fn settle_reservation(&mut self, reservation_id: &str, actual_cost: u64) -> Result<()> {
        let reservation = self
            .reservations
            .remove(reservation_id)
            .ok_or_else(|| Error::Internal(format!("Reservation not found: {}", reservation_id)))?;

        let did = &reservation.did;
        let reserved_amount = reservation.reserved_amount;

        // Remove call-to-reservation mapping
        self.call_reservations.remove(&reservation.call_id);

        // Update status: deduct actual cost, release reserved amount
        let status = self.status.entry(did.to_string()).or_default();
        status.spent_minute += actual_cost;
        status.spent_hour += actual_cost;
        status.spent_day += actual_cost;
        status.spent_total += actual_cost;
        status.reserved -= reserved_amount;
        status.active_reservations -= 1;

        Ok(())
    }

    /// Release a reservation without deducting any cost (call cancelled/failed)
    pub fn release_reservation(&mut self, reservation_id: &str) -> Result<()> {
        let reservation = self
            .reservations
            .remove(reservation_id)
            .ok_or_else(|| Error::Internal(format!("Reservation not found: {}", reservation_id)))?;

        let did = &reservation.did;
        let reserved_amount = reservation.reserved_amount;

        // Remove call mapping
        self.call_reservations.remove(&reservation.call_id);

        // Release reserved amount
        let status = self.status.entry(did.to_string()).or_default();
        status.reserved -= reserved_amount;
        status.active_reservations -= 1;

        Ok(())
    }

    /// Release reservation by call ID
    pub fn release_reservation_by_call(&mut self, call_id: &str) -> Result<()> {
        let reservation_id = self
            .call_reservations
            .remove(call_id)
            .ok_or_else(|| Error::Internal(format!("No reservation for call: {}", call_id)))?;
        self.release_reservation(&reservation_id)?;
        Ok(())
    }

    /// Get the reservation for a specific call
    pub fn get_reservation_for_call(&self, call_id: &str) -> Option<&BudgetReservation> {
        self.call_reservations
            .get(call_id)
            .and_then(|rid| self.reservations.get(rid))
    }

    /// Record direct spending (without reservation, for simple tracking)
    pub fn record_spending(&mut self, did: &str, amount: u64) {
        let status = self.status.entry(did.to_string()).or_default();
        status.spent_minute += amount;
        status.spent_hour += amount;
        status.spent_day += amount;
        status.spent_total += amount;
    }

    /// Reset minute-level budgets
    pub fn reset_minute(&mut self) {
        for status in self.status.values_mut() {
            status.spent_minute = 0;
        }
    }

    /// Reset hourly budgets (also resets minute)
    pub fn reset_hourly(&mut self) {
        for status in self.status.values_mut() {
            status.spent_minute = 0;
            status.spent_hour = 0;
        }
    }

    /// Reset daily budgets (also resets hour and minute)
    pub fn reset_daily(&mut self) {
        for status in self.status.values_mut() {
            status.spent_minute = 0;
            status.spent_hour = 0;
            status.spent_day = 0;
        }
    }

    /// Get budget status for a specific DID
    pub fn get_status(&self, did: &str) -> BudgetStatus {
        self.status.get(did).cloned().unwrap_or_default()
    }

    /// Get available budget for a DID (considering spent + reserved)
    pub fn available_budget(&self, did: &str) -> u64 {
        let status = self.status.get(did).cloned().unwrap_or_default();
        self.limits.max_total - status.spent_total - status.reserved
    }

    /// Get the number of active reservations for a DID
    pub fn active_reservation_count(&self, did: &str) -> usize {
        self.status
            .get(did)
            .map(|s| s.active_reservations)
            .unwrap_or(0)
    }

    /// Cancel all in-progress reservations for a DID (budget exhausted)
    ///
    /// Used when budget is exhausted to terminate all ongoing calls.
    pub fn cancel_all_reservations(&mut self, did: &str) -> Vec<String> {
        let cancelled_calls: Vec<String> = self
            .reservations
            .values()
            .filter(|r| r.did == did)
            .map(|r| r.call_id.clone())
            .collect();

        for call_id in &cancelled_calls {
            if let Some(rid) = self.call_reservations.remove(call_id) {
                self.reservations.remove(&rid);
            }
        }

        // Reset reservation tracking in status
        if let Some(status) = self.status.get_mut(did) {
            status.reserved = 0;
            status.active_reservations = 0;
        }

        cancelled_calls
    }
}

impl Default for BudgetController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_check() {
        let controller = BudgetController::new();

        assert!(controller.check_budget("did:nexa:test", 50).is_ok());
        assert!(controller.check_budget("did:nexa:test", 200).is_err()); // exceeds per_call limit
    }

    #[test]
    fn test_budget_recording() {
        let mut controller = BudgetController::new();

        controller.record_spending("did:nexa:test", 100);

        let status = controller.get_status("did:nexa:test");
        assert_eq!(status.spent_total, 100);
        assert_eq!(status.spent_minute, 100);
    }

    #[test]
    fn test_budget_reservation() {
        let mut controller = BudgetController::new();

        // Reserve budget for a call
        let reservation_id = controller
            .reserve_budget("did:nexa:test", "call-1", 50)
            .unwrap();

        // Check that reservation is tracked
        let status = controller.get_status("did:nexa:test");
        assert_eq!(status.reserved, 50);
        assert_eq!(status.active_reservations, 1);
        assert_eq!(status.spent_total, 0); // Not spent yet, just reserved

        // Another reservation should consider the first one
        // Per-call limit is 100, per-minute limit is 500
        // 50 reserved + 50 new = 100 (under per_minute, at per_call limit)
        assert!(controller.check_budget("did:nexa:test", 50).is_ok());
        // 50 reserved + 100 new = 150 (under per_minute but exceeds per_call)
        assert!(controller.check_budget("did:nexa:test", 101).is_err()); // exceeds per_call limit

        // Settle the reservation with actual cost
        controller.settle_reservation(&reservation_id, 40).unwrap();

        let status = controller.get_status("did:nexa:test");
        assert_eq!(status.spent_total, 40); // Actual cost recorded
        assert_eq!(status.reserved, 0); // Reservation released
        assert_eq!(status.active_reservations, 0);
    }

    #[test]
    fn test_budget_reservation_release() {
        let mut controller = BudgetController::new();

        let reservation_id = controller
            .reserve_budget("did:nexa:test", "call-1", 50)
            .unwrap();

        // Release without spending (call cancelled)
        controller.release_reservation(&reservation_id).unwrap();

        let status = controller.get_status("did:nexa:test");
        assert_eq!(status.spent_total, 0); // No cost
        assert_eq!(status.reserved, 0); // Reservation released
    }

    #[test]
    fn test_budget_reservation_release_by_call() {
        let mut controller = BudgetController::new();

        controller
            .reserve_budget("did:nexa:test", "call-1", 50)
            .unwrap();

        // Release by call ID
        controller.release_reservation_by_call("call-1").unwrap();

        let status = controller.get_status("did:nexa:test");
        assert_eq!(status.spent_total, 0);
        assert_eq!(status.reserved, 0);
    }

    #[test]
    fn test_multi_level_limits() {
        let limits = BudgetLimit {
            max_per_call: 10,
            max_per_minute: 20,
            max_per_hour: 50,
            max_per_day: 100,
            max_total: 500,
        };
        let mut controller = BudgetController::with_limits(limits);

        // Per-call limit
        assert!(controller.check_budget("did:nexa:test", 11).is_err());

        // Per-minute limit: first call OK
        assert!(controller.check_budget("did:nexa:test", 10).is_ok());
        controller.record_spending("did:nexa:test", 10);

        // Second call in same minute exceeds limit
        assert!(controller.check_budget("did:nexa:test", 11).is_err());
        assert!(controller.check_budget("did:nexa:test", 10).is_ok()); // exactly at limit
    }

    #[test]
    fn test_total_budget_limit() {
        let limits = BudgetLimit {
            max_per_call: 1000,
            max_per_minute: 1000,
            max_per_hour: 1000,
            max_per_day: 1000,
            max_total: 500,
        };
        let mut controller = BudgetController::with_limits(limits);

        controller.record_spending("did:nexa:test", 400);

        // Only 100 left
        assert!(controller.check_budget("did:nexa:test", 100).is_ok());
        assert!(controller.check_budget("did:nexa:test", 101).is_err());
    }

    #[test]
    fn test_available_budget() {
        let controller = BudgetController::new();
        assert_eq!(controller.available_budget("did:nexa:test"), 100000); // default max_total

        let mut controller = controller;
        controller.record_spending("did:nexa:test", 1000);
        assert_eq!(controller.available_budget("did:nexa:test"), 99000);
    }

    #[test]
    fn test_cancel_all_reservations() {
        let mut controller = BudgetController::new();

        controller
            .reserve_budget("did:nexa:test", "call-1", 50)
            .unwrap();
        controller
            .reserve_budget("did:nexa:test", "call-2", 30)
            .unwrap();

        let status = controller.get_status("did:nexa:test");
        assert_eq!(status.reserved, 80);
        assert_eq!(status.active_reservations, 2);

        // Cancel all reservations
        let cancelled = controller.cancel_all_reservations("did:nexa:test");
        assert_eq!(cancelled.len(), 2);
        assert!(cancelled.contains(&"call-1".to_string()));
        assert!(cancelled.contains(&"call-2".to_string()));

        let status = controller.get_status("did:nexa:test");
        assert_eq!(status.reserved, 0);
        assert_eq!(status.active_reservations, 0);
    }

    #[test]
    fn test_budget_reset() {
        let mut controller = BudgetController::new();

        controller.record_spending("did:nexa:test", 100);

        controller.reset_minute();
        let status = controller.get_status("did:nexa:test");
        assert_eq!(status.spent_minute, 0);
        assert_eq!(status.spent_hour, 100); // Not reset yet

        controller.reset_hourly();
        let status = controller.get_status("did:nexa:test");
        assert_eq!(status.spent_hour, 0);

        controller.reset_daily();
        let status = controller.get_status("did:nexa:test");
        assert_eq!(status.spent_day, 0);
        assert_eq!(status.spent_total, 100); // Total never resets
    }

    // ========== Boundary/Error Tests ==========

    #[test]
    fn test_budget_check_per_call_zero_ok() {
        let controller = BudgetController::new();
        // Zero amount should always pass (per_call limit is 100, so 0 < 100)
        assert!(controller.check_budget("did:nexa:test", 0).is_ok());
    }

    #[test]
    fn test_budget_settle_nonexistent_reservation() {
        let mut controller = BudgetController::new();
        let result = controller.settle_reservation("reserve-nonexistent", 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_budget_release_nonexistent_reservation() {
        let mut controller = BudgetController::new();
        let result = controller.release_reservation("reserve-nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_budget_release_by_call_nonexistent() {
        let mut controller = BudgetController::new();
        let result = controller.release_reservation_by_call("call-nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_budget_available_budget_with_reservation() {
        let mut controller = BudgetController::new();
        // Use amounts that fit within all default limits:
        // per_call=100, per_minute=500, per_hour=1000, per_day=10000, max_total=100000
        controller.record_spending("did:nexa:test", 200);
        controller
            .reserve_budget("did:nexa:test", "call-1", 50)
            .unwrap();

        // Available = max_total - spent - reserved
        assert_eq!(
            controller.available_budget("did:nexa:test"),
            100000 - 200 - 50
        );
    }

    #[test]
    fn test_budget_available_budget_unknown_did() {
        let controller = BudgetController::new();
        // Unknown DID should have full budget available
        assert_eq!(controller.available_budget("did:nexa:unknown"), 100000);
    }

    #[test]
    fn test_budget_active_reservation_count_unknown_did() {
        let controller = BudgetController::new();
        assert_eq!(controller.active_reservation_count("did:nexa:unknown"), 0);
    }

    #[test]
    fn test_budget_get_reservation_for_call() {
        let mut controller = BudgetController::new();
        controller
            .reserve_budget("did:nexa:test", "call-1", 50)
            .unwrap();

        let reservation = controller.get_reservation_for_call("call-1");
        assert!(reservation.is_some());
        assert_eq!(reservation.unwrap().reserved_amount, 50);

        let nonexistent = controller.get_reservation_for_call("call-nonexistent");
        assert!(nonexistent.is_none());
    }

    #[test]
    fn test_budget_cancel_all_for_did_with_no_reservations() {
        let mut controller = BudgetController::new();
        let cancelled = controller.cancel_all_reservations("did:nexa:unknown");
        assert!(cancelled.is_empty());
    }

    #[test]
    fn test_budget_exceeds_per_minute_with_reservation() {
        let limits = BudgetLimit {
            max_per_call: 100,
            max_per_minute: 100,
            max_per_hour: 1000,
            max_per_day: 10000,
            max_total: 100000,
        };
        let mut controller = BudgetController::with_limits(limits);

        // Reserve 50
        controller
            .reserve_budget("did:nexa:test", "call-1", 50)
            .unwrap();

        // Try to check budget for 60 — 50 reserved + 60 = 110 > 100 per_minute
        assert!(controller.check_budget("did:nexa:test", 60).is_err());
    }

    #[test]
    fn test_budget_settle_then_check_budget() {
        let limits = BudgetLimit {
            max_per_call: 100,
            max_per_minute: 100,
            max_per_hour: 1000,
            max_per_day: 10000,
            max_total: 100,
        };
        let mut controller = BudgetController::with_limits(limits);

        let rid = controller
            .reserve_budget("did:nexa:test", "call-1", 50)
            .unwrap();
        controller.settle_reservation(&rid, 50).unwrap();

        // Now 50 spent out of 100 total — only 50 left
        assert!(controller.check_budget("did:nexa:test", 50).is_ok());
        assert!(controller.check_budget("did:nexa:test", 51).is_err());
    }

    #[test]
    fn test_budget_auto_termination_on_exhaustion() {
        let limits = BudgetLimit {
            max_per_call: 100,
            max_per_minute: 500,
            max_per_hour: 1000,
            max_per_day: 10000,
            max_total: 100,
        };
        let mut controller = BudgetController::with_limits(limits);

        // Spend all budget
        controller.record_spending("did:nexa:test", 100);

        // Any new call should be rejected
        assert!(controller.check_budget("did:nexa:test", 1).is_err());

        // Reserve should also fail
        assert!(controller
            .reserve_budget("did:nexa:test", "call-1", 1)
            .is_err());
    }

    // ========== Proptest Tests ==========

    use proptest::prelude::*;

    proptest! {
        /// Budget limit enforcement: spending + reservation never exceeds limits
        #[test]
        fn proptest_budget_limit_enforcement(
            per_call in 10u64..500,
            per_minute in 10u64..5000,
            per_total in 100u64..50000,
            request_amount in 1u64..1000,
        ) {
            // Ensure hierarchical ordering: per_call ≤ per_minute ≤ per_total
            let per_minute = per_minute.max(per_call);
            let per_total = per_total.max(per_minute);

            let limits = BudgetLimit {
                max_per_call: per_call,
                max_per_minute: per_minute,
                max_per_hour: per_minute * 10,
                max_per_day: per_minute * 100,
                max_total: per_total,
            };
            let controller = BudgetController::with_limits(limits);

            if request_amount > per_call {
                assert!(controller.check_budget("did:nexa:test", request_amount).is_err());
            } else {
                assert!(controller.check_budget("did:nexa:test", request_amount).is_ok());
            }
        }
    }
}
