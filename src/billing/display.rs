//! Batched real-time cost display.
//!
//! Accumulates task costs and provides periodic display updates rather
//! than emitting an event on every single charge. This keeps UI overhead
//! low while giving users a near-real-time view of their spending.
//!
//! # Design
//!
//! The [`CostDisplay`] struct tracks accumulated costs since the last
//! flush. Callers should flush on a timer (e.g. every 2-5 seconds)
//! and on session end. The flush returns a [`DisplayUpdate`] only if
//! there is new activity to report.

use super::spending::{format_cents, WarningLevel};

/// A batched cost display update.
#[derive(Debug, Clone)]
pub struct DisplayUpdate {
    /// Formatted current balance (e.g. "$7.40").
    pub balance: String,
    /// Number of tasks completed since last update.
    pub tasks_since_last: u32,
    /// Cost accumulated since last update in cents.
    pub cost_since_last_cents: u32,
    /// Formatted cost since last update (e.g. "$0.30").
    pub cost_since_last: String,
    /// Session totals.
    pub session_tasks: u32,
    /// Session total cost in cents.
    pub session_cost_cents: u32,
    /// Formatted session total cost.
    pub session_cost: String,
    /// Current warning level.
    pub warning: WarningLevel,
}

/// Batched cost display accumulator.
///
/// Call [`record`] after each task charge and [`flush`] periodically
/// to get display updates.
#[derive(Debug)]
pub struct CostDisplay {
    /// Tasks since last flush.
    pending_tasks: u32,
    /// Cost since last flush in cents.
    pending_cost_cents: u32,
    /// Total tasks this session.
    session_tasks: u32,
    /// Total cost this session in cents.
    session_cost_cents: u32,
}

impl CostDisplay {
    pub fn new() -> Self {
        Self {
            pending_tasks: 0,
            pending_cost_cents: 0,
            session_tasks: 0,
            session_cost_cents: 0,
        }
    }

    /// Record a completed task charge.
    pub fn record(&mut self, cost_cents: u32) {
        self.pending_tasks += 1;
        self.pending_cost_cents += cost_cents;
        self.session_tasks += 1;
        self.session_cost_cents += cost_cents;
    }

    /// Flush pending activity into a display update.
    ///
    /// Returns `None` if there is no new activity since the last flush.
    /// The caller provides the current balance and warning level.
    pub fn flush(
        &mut self,
        current_balance_cents: i64,
        warning: WarningLevel,
    ) -> Option<DisplayUpdate> {
        if self.pending_tasks == 0 {
            return None;
        }

        let update = DisplayUpdate {
            balance: format_cents(current_balance_cents),
            tasks_since_last: self.pending_tasks,
            cost_since_last_cents: self.pending_cost_cents,
            cost_since_last: format_cents(i64::from(self.pending_cost_cents)),
            session_tasks: self.session_tasks,
            session_cost_cents: self.session_cost_cents,
            session_cost: format_cents(i64::from(self.session_cost_cents)),
            warning,
        };

        self.pending_tasks = 0;
        self.pending_cost_cents = 0;

        Some(update)
    }

    /// Whether there is pending activity to flush.
    pub fn has_pending(&self) -> bool {
        self.pending_tasks > 0
    }

    /// Total tasks this session.
    pub fn session_tasks(&self) -> u32 {
        self.session_tasks
    }

    /// Total cost this session in cents.
    pub fn session_cost_cents(&self) -> u32 {
        self.session_cost_cents
    }

    /// Reset session totals (e.g. on new session).
    pub fn reset_session(&mut self) {
        self.pending_tasks = 0;
        self.pending_cost_cents = 0;
        self.session_tasks = 0;
        self.session_cost_cents = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_display_is_empty() {
        let display = CostDisplay::new();
        assert_eq!(display.session_tasks(), 0);
        assert_eq!(display.session_cost_cents(), 0);
        assert!(!display.has_pending());
    }

    #[test]
    fn record_accumulates() {
        let mut display = CostDisplay::new();
        display.record(10);
        display.record(10);
        display.record(10);

        assert!(display.has_pending());
        assert_eq!(display.session_tasks(), 3);
        assert_eq!(display.session_cost_cents(), 30);
    }

    #[test]
    fn flush_returns_update_and_resets_pending() {
        let mut display = CostDisplay::new();
        display.record(10);
        display.record(10);

        let update = display.flush(980, WarningLevel::None).unwrap();
        assert_eq!(update.tasks_since_last, 2);
        assert_eq!(update.cost_since_last_cents, 20);
        assert_eq!(update.cost_since_last, "$0.20");
        assert_eq!(update.balance, "$9.80");
        assert_eq!(update.session_tasks, 2);
        assert_eq!(update.session_cost_cents, 20);
        assert_eq!(update.session_cost, "$0.20");
        assert_eq!(update.warning, WarningLevel::None);

        // Pending is cleared, session totals remain
        assert!(!display.has_pending());
        assert_eq!(display.session_tasks(), 2);
    }

    #[test]
    fn flush_returns_none_when_no_activity() {
        let mut display = CostDisplay::new();
        assert!(display.flush(1000, WarningLevel::None).is_none());
    }

    #[test]
    fn flush_with_warning() {
        let mut display = CostDisplay::new();
        display.record(10);
        let update = display.flush(150, WarningLevel::Low).unwrap();
        assert_eq!(update.warning, WarningLevel::Low);
    }

    #[test]
    fn multiple_flushes_accumulate_session() {
        let mut display = CostDisplay::new();

        // First batch: 3 tasks
        display.record(10);
        display.record(10);
        display.record(10);
        let update1 = display.flush(970, WarningLevel::None).unwrap();
        assert_eq!(update1.tasks_since_last, 3);
        assert_eq!(update1.session_tasks, 3);

        // Second batch: 2 tasks
        display.record(10);
        display.record(10);
        let update2 = display.flush(950, WarningLevel::None).unwrap();
        assert_eq!(update2.tasks_since_last, 2);
        assert_eq!(update2.session_tasks, 5);
        assert_eq!(update2.session_cost_cents, 50);
    }

    #[test]
    fn reset_session_clears_everything() {
        let mut display = CostDisplay::new();
        display.record(10);
        display.record(10);

        display.reset_session();
        assert!(!display.has_pending());
        assert_eq!(display.session_tasks(), 0);
        assert_eq!(display.session_cost_cents(), 0);
    }
}
