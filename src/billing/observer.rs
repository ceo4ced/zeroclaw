//! Billing observer for agent lifecycle integration.
//!
//! Implements the [`Observer`] trait to automatically meter tasks as
//! they flow through the agent orchestration loop. Listens for
//! `AgentStart` and `AgentEnd` events to track task boundaries and
//! charge the flat per-task rate.
//!
//! # Integration
//!
//! Register this observer alongside other observers (log, prometheus,
//! etc.) during startup. It will automatically:
//! 1. Run a preflight check on `AgentStart` (logs warning if blocked).
//! 2. Charge the flat task rate on `AgentEnd`.
//! 3. Update the batched cost display.
//! 4. Persist balance changes to disk.

use parking_lot::Mutex;
use std::path::PathBuf;

use crate::billing::account::UserAccount;
use crate::billing::config::BillingConfig;
use crate::billing::display::CostDisplay;
use crate::billing::persistence;
use crate::billing::spending::WarningLevel;
use crate::observability::traits::{Observer, ObserverEvent, ObserverMetric};

/// Observer that meters tasks and manages billing state.
pub struct BillingObserver {
    /// Protected billing state.
    state: Mutex<BillingState>,
    /// Whether billing is enabled.
    enabled: bool,
}

/// Internal mutable billing state.
struct BillingState {
    account: UserAccount,
    display: CostDisplay,
    workspace_dir: PathBuf,
    /// Whether a task is currently in flight (between AgentStart and AgentEnd).
    task_in_flight: bool,
}

impl BillingObserver {
    /// Create a new billing observer from config.
    ///
    /// Loads persisted balance from disk if available. If billing is
    /// disabled in config, creates a no-op observer that passes all events through.
    pub fn new(config: &BillingConfig, user_id: &str, workspace_dir: &std::path::Path) -> Self {
        let mut account = UserAccount::new(user_id, config);

        // Try to load persisted balance
        if let Ok(persisted) = persistence::load_balance(workspace_dir, user_id) {
            account.restore_balance(persisted.balance_cents);
        }

        Self {
            state: Mutex::new(BillingState {
                account,
                display: CostDisplay::new(),
                workspace_dir: workspace_dir.to_path_buf(),
                task_in_flight: false,
            }),
            enabled: config.enabled,
        }
    }

    /// Get the current account summary (for CLI display).
    pub fn account_summary(&self) -> crate::billing::account::AccountSummary {
        let state = self.state.lock();
        state.account.summary()
    }

    /// Top up the account balance.
    pub fn top_up(
        &self,
        amount_cents: u32,
    ) -> anyhow::Result<crate::billing::account::TopUpResult> {
        let mut state = self.state.lock();
        let result = state.account.top_up(amount_cents)?;
        // Persist immediately after top-up
        let _ = persistence::save_balance(
            &state.workspace_dir,
            &state.account.user_id,
            state.account.balance_cents(),
        );
        Ok(result)
    }

    /// Set daily spending limit.
    pub fn set_daily_limit(&self, limit_cents: u32) {
        let mut state = self.state.lock();
        state.account.set_daily_limit(limit_cents);
    }

    /// Reset daily counters (should be called at day boundary by cron).
    pub fn reset_daily(&self) {
        let mut state = self.state.lock();
        state.account.reset_daily();
    }

    /// Flush the cost display and return any pending update.
    pub fn flush_display(&self) -> Option<crate::billing::display::DisplayUpdate> {
        let mut state = self.state.lock();
        let balance = state.account.balance_cents();
        let warning = if balance <= 100 {
            WarningLevel::Critical
        } else if balance <= 200 {
            WarningLevel::Low
        } else {
            WarningLevel::None
        };
        state.display.flush(balance, warning)
    }
}

impl Observer for BillingObserver {
    fn record_event(&self, event: &ObserverEvent) {
        if !self.enabled {
            return;
        }

        match event {
            ObserverEvent::AgentStart { .. } => {
                let mut state = self.state.lock();
                state.task_in_flight = true;

                // Preflight check — log warning if task might be blocked
                let preflight = state.account.start_task();
                if !preflight.allowed {
                    tracing::warn!(
                        balance = %preflight.balance_display,
                        reason = preflight.reason.as_deref().unwrap_or("unknown"),
                        "Billing preflight check failed"
                    );
                }
                if preflight.warning != WarningLevel::None {
                    tracing::warn!(
                        balance = %preflight.balance_display,
                        level = %preflight.warning,
                        "Low balance warning"
                    );
                }
            }
            ObserverEvent::AgentEnd { .. } => {
                let mut state = self.state.lock();
                if !state.task_in_flight {
                    return;
                }
                state.task_in_flight = false;

                // Charge for completed task
                let charge = state.account.complete_task();
                if charge.charged {
                    state.display.record(charge.amount_cents);
                    tracing::debug!(
                        amount = %charge.amount_cents,
                        balance = %charge.balance_display,
                        warning = %charge.warning,
                        "Task charged"
                    );

                    // Persist balance change
                    let _ = persistence::save_balance(
                        &state.workspace_dir,
                        &state.account.user_id,
                        charge.new_balance_cents,
                    );
                } else if let Some(reason) = &charge.rejection_reason {
                    tracing::error!(
                        balance = %charge.balance_display,
                        reason = %reason,
                        "Failed to charge for task"
                    );
                }
            }
            _ => {}
        }
    }

    fn record_metric(&self, _metric: &ObserverMetric) {
        // Billing doesn't consume metrics
    }

    fn flush(&self) {
        if !self.enabled {
            return;
        }
        // Persist final balance on shutdown
        let state = self.state.lock();
        let _ = persistence::save_balance(
            &state.workspace_dir,
            &state.account.user_id,
            state.account.balance_cents(),
        );
    }

    fn name(&self) -> &str {
        "billing"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;

    fn test_observer(enabled: bool) -> (BillingObserver, TempDir) {
        let tmp = TempDir::new().unwrap();
        let mut config = BillingConfig::default();
        config.enabled = enabled;
        let observer = BillingObserver::new(&config, "test_user", tmp.path());
        (observer, tmp)
    }

    #[test]
    fn disabled_observer_passes_through() {
        let (observer, _tmp) = test_observer(false);
        observer.record_event(&ObserverEvent::AgentStart {
            provider: "test".into(),
            model: "test".into(),
        });
        observer.record_event(&ObserverEvent::AgentEnd {
            provider: "test".into(),
            model: "test".into(),
            duration: Duration::from_secs(1),
            tokens_used: None,
            cost_usd: None,
        });
        // No charge since disabled
        let summary = observer.account_summary();
        assert_eq!(summary.tasks_today, 0);
    }

    #[test]
    fn enabled_observer_charges_on_agent_end() {
        let (observer, _tmp) = test_observer(true);
        // Top up first
        observer.top_up(1000).unwrap();

        observer.record_event(&ObserverEvent::AgentStart {
            provider: "test".into(),
            model: "test".into(),
        });
        observer.record_event(&ObserverEvent::AgentEnd {
            provider: "test".into(),
            model: "test".into(),
            duration: Duration::from_secs(1),
            tokens_used: None,
            cost_usd: None,
        });

        let summary = observer.account_summary();
        assert_eq!(summary.tasks_today, 1);
        assert_eq!(summary.balance_cents, 990);
    }

    #[test]
    fn observer_persists_balance() {
        let tmp = TempDir::new().unwrap();
        let mut config = BillingConfig::default();
        config.enabled = true;

        // Create, top up, run a task
        {
            let observer = BillingObserver::new(&config, "test_user", tmp.path());
            observer.top_up(1000).unwrap();
            observer.record_event(&ObserverEvent::AgentStart {
                provider: "test".into(),
                model: "test".into(),
            });
            observer.record_event(&ObserverEvent::AgentEnd {
                provider: "test".into(),
                model: "test".into(),
                duration: Duration::from_secs(1),
                tokens_used: None,
                cost_usd: None,
            });
            observer.flush();
        }

        // Reload — balance should be persisted
        let observer2 = BillingObserver::new(&config, "test_user", tmp.path());
        let summary = observer2.account_summary();
        assert_eq!(summary.balance_cents, 990);
    }

    #[test]
    fn observer_name() {
        let (observer, _tmp) = test_observer(true);
        assert_eq!(observer.name(), "billing");
    }

    #[test]
    fn no_charge_without_agent_start() {
        let (observer, _tmp) = test_observer(true);
        observer.top_up(1000).unwrap();

        // AgentEnd without AgentStart — should not charge
        observer.record_event(&ObserverEvent::AgentEnd {
            provider: "test".into(),
            model: "test".into(),
            duration: Duration::from_secs(1),
            tokens_used: None,
            cost_usd: None,
        });

        let summary = observer.account_summary();
        assert_eq!(summary.tasks_today, 0);
        assert_eq!(summary.balance_cents, 1000);
    }

    #[test]
    fn flush_display_returns_update() {
        let (observer, _tmp) = test_observer(true);
        observer.top_up(1000).unwrap();

        observer.record_event(&ObserverEvent::AgentStart {
            provider: "test".into(),
            model: "test".into(),
        });
        observer.record_event(&ObserverEvent::AgentEnd {
            provider: "test".into(),
            model: "test".into(),
            duration: Duration::from_secs(1),
            tokens_used: None,
            cost_usd: None,
        });

        let update = observer.flush_display();
        assert!(update.is_some());
        let update = update.unwrap();
        assert_eq!(update.session_tasks, 1);
        assert_eq!(update.session_cost_cents, 10);
    }
}
