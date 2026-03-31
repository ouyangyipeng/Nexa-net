//! Budget Controller
//!
//! Budget control and resource guardrails.

use crate::error::{Error, Result};
use std::collections::HashMap;

/// Budget limit
#[derive(Debug, Clone)]
pub struct BudgetLimit {
    /// Maximum budget per call
    pub max_per_call: u64,
    /// Maximum budget per hour
    pub max_per_hour: u64,
    /// Maximum budget per day
    pub max_per_day: u64,
}

impl Default for BudgetLimit {
    fn default() -> Self {
        Self {
            max_per_call: 100,
            max_per_hour: 1000,
            max_per_day: 10000,
        }
    }
}

/// Budget status
#[derive(Debug, Clone, Default)]
pub struct BudgetStatus {
    /// Spent in current hour
    pub spent_hour: u64,
    /// Spent in current day
    pub spent_day: u64,
    /// Total spent
    pub spent_total: u64,
}

/// Budget controller
#[derive(Debug, Clone)]
pub struct BudgetController {
    /// Budget limits
    pub limits: BudgetLimit,
    /// Spending status by DID
    status: HashMap<String, BudgetStatus>,
}

impl BudgetController {
    /// Create a new budget controller
    pub fn new() -> Self {
        Self {
            limits: BudgetLimit::default(),
            status: HashMap::new(),
        }
    }

    /// Check if a call is within budget
    pub fn check_budget(&self, did: &str, amount: u64) -> Result<()> {
        let status = self.status.get(did).cloned().unwrap_or_default();

        if amount > self.limits.max_per_call {
            return Err(Error::BudgetExceeded(amount, self.limits.max_per_call));
        }

        if status.spent_hour + amount > self.limits.max_per_hour {
            return Err(Error::BudgetExceeded(
                status.spent_hour + amount,
                self.limits.max_per_hour,
            ));
        }

        if status.spent_day + amount > self.limits.max_per_day {
            return Err(Error::BudgetExceeded(
                status.spent_day + amount,
                self.limits.max_per_day,
            ));
        }

        Ok(())
    }

    /// Record spending
    pub fn record_spending(&mut self, did: &str, amount: u64) {
        let status = self.status.entry(did.to_string()).or_default();
        status.spent_hour += amount;
        status.spent_day += amount;
        status.spent_total += amount;
    }

    /// Reset hourly budgets
    pub fn reset_hourly(&mut self) {
        for status in self.status.values_mut() {
            status.spent_hour = 0;
        }
    }

    /// Reset daily budgets
    pub fn reset_daily(&mut self) {
        for status in self.status.values_mut() {
            status.spent_hour = 0;
            status.spent_day = 0;
        }
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
        assert!(controller.check_budget("did:nexa:test", 200).is_err());
    }

    #[test]
    fn test_budget_recording() {
        let mut controller = BudgetController::new();

        controller.record_spending("did:nexa:test", 100);

        let status = controller.status.get("did:nexa:test").unwrap();
        assert_eq!(status.spent_total, 100);
    }
}
