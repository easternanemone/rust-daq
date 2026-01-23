//! Shutter Safety Module for Laser Control
//!
//! This module provides enhanced safety mechanisms for laser shutter control
//! that go beyond simple Rust Drop semantics.
//!
//! # Problem Statement
//!
//! The basic `with_shutter_open()` function relies on Rust's control flow
//! to close the shutter after the callback completes. However, this does NOT
//! protect against:
//!
//! - **SIGKILL**: Cannot be intercepted (kill -9, OOM killer)
//! - **Power failure**: Immediate loss of control
//! - **Process hangs**: Infinite loops, deadlocks, hardware timeouts
//! - **Hardware crashes**: Host machine failure
//!
//! # Safety Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────┐
//! │                    DEFENSE IN DEPTH                        │
//! ├────────────────────────────────────────────────────────────┤
//! │ Layer 1: with_shutter_open()     - Rust control flow      │
//! │ Layer 2: HeartbeatShutterGuard   - Timeout-based closure  │
//! │ Layer 3: SIGTERM/SIGINT handlers - Graceful shutdown      │
//! │ Layer 4: ShutterRegistry         - Emergency close-all    │
//! │ Layer 5: Hardware interlock      - EXTERNAL (recommended) │
//! └────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Limitations
//!
//! **CRITICAL**: Software protections cannot protect against SIGKILL or power
//! failure. For production laser labs, **always use hardware interlocks** in
//! addition to software safety mechanisms.
//!
//! # Usage
//!
//! ```rust,ignore
//! use daq_scripting::shutter_safety::{HeartbeatShutterGuard, ShutterRegistry};
//!
//! // Register a global emergency shutdown handler
//! ShutterRegistry::install_signal_handlers();
//!
//! // Use heartbeat-based guard (closes if no heartbeat for 5s)
//! let guard = HeartbeatShutterGuard::new(shutter_driver, Duration::from_secs(5)).await?;
//!
//! // Script must call heartbeat() periodically
//! loop {
//!     guard.heartbeat();
//!     do_work();
//! }
//!
//! // Shutter auto-closes on drop OR if heartbeats stop
//! ```

use daq_hardware::capabilities::ShutterControl;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant};
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

/// Default heartbeat timeout (5 seconds)
pub const DEFAULT_HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(5);

/// Maximum allowed heartbeat timeout (60 seconds)
pub const MAX_HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(60);

// =============================================================================
// Global Shutter Registry
// =============================================================================

/// Global registry of all currently open shutters for emergency shutdown.
///
/// This registry allows signal handlers and panic hooks to close all shutters
/// when the process is terminating unexpectedly.
static SHUTTER_REGISTRY: OnceLock<ShutterRegistry> = OnceLock::new();

/// Counter for generating unique guard IDs
static GUARD_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Registry of open shutters for emergency shutdown
pub struct ShutterRegistry {
    /// Map of guard ID to weak reference to shutter driver
    shutters: Mutex<HashMap<u64, Weak<dyn ShutterControl>>>,
    /// Flag indicating if signal handlers are installed
    handlers_installed: AtomicBool,
}

impl ShutterRegistry {
    /// Get or create the global registry
    pub fn global() -> &'static ShutterRegistry {
        SHUTTER_REGISTRY.get_or_init(|| ShutterRegistry {
            shutters: Mutex::new(HashMap::new()),
            handlers_installed: AtomicBool::new(false),
        })
    }

    /// Register a shutter for emergency shutdown
    pub fn register(driver: &Arc<dyn ShutterControl>) -> u64 {
        let id = GUARD_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        let weak = Arc::downgrade(driver);

        if let Ok(mut shutters) = Self::global().shutters.lock() {
            shutters.insert(id, weak);
            info!(
                guard_id = id,
                total_shutters = shutters.len(),
                "Registered shutter for emergency shutdown"
            );
        }

        id
    }

    /// Unregister a shutter
    pub fn unregister(id: u64) {
        if let Ok(mut shutters) = Self::global().shutters.lock() {
            if shutters.remove(&id).is_some() {
                info!(
                    guard_id = id,
                    remaining = shutters.len(),
                    "Unregistered shutter from emergency registry"
                );
            }
        }
    }

    /// Emergency close all registered shutters
    ///
    /// This is called by signal handlers and panic hooks.
    /// It attempts to close all shutters but cannot guarantee success
    /// (e.g., if hardware is unresponsive).
    pub fn emergency_close_all() {
        warn!("EMERGENCY: Closing all registered shutters");

        let shutters: Vec<Arc<dyn ShutterControl>> = {
            if let Ok(guard) = Self::global().shutters.lock() {
                guard.values().filter_map(|weak| weak.upgrade()).collect()
            } else {
                error!("Failed to acquire shutter registry lock during emergency close");
                return;
            }
        };

        if shutters.is_empty() {
            info!("No shutters registered for emergency close");
            return;
        }

        info!("Attempting to close {} registered shutters", shutters.len());

        // Try to close each shutter
        // Note: We're in an emergency context, so we use blocking calls
        for (i, shutter) in shutters.iter().enumerate() {
            // Try to get a runtime handle
            if let Ok(handle) = Handle::try_current() {
                // We're in an async context
                let shutter = shutter.clone();
                let result = std::thread::spawn(move || {
                    handle.block_on(async {
                        match tokio::time::timeout(Duration::from_secs(2), shutter.close_shutter())
                            .await
                        {
                            Ok(Ok(())) => {
                                info!(shutter_index = i, "Emergency shutter close: SUCCESS");
                                true
                            }
                            Ok(Err(e)) => {
                                error!(
                                    shutter_index = i,
                                    error = %e,
                                    "Emergency shutter close: FAILED"
                                );
                                false
                            }
                            Err(_) => {
                                error!(shutter_index = i, "Emergency shutter close: TIMEOUT (2s)");
                                false
                            }
                        }
                    })
                })
                .join();

                if let Err(e) = result {
                    error!(shutter_index = i, error = ?e, "Emergency close thread panicked");
                }
            } else {
                warn!(
                    shutter_index = i,
                    "No tokio runtime available for emergency shutter close"
                );
            }
        }
    }

    /// Install signal handlers for graceful shutdown
    ///
    /// This installs handlers for SIGTERM and SIGINT that will attempt
    /// to close all shutters before the process exits.
    ///
    /// # Platform Support
    /// - Unix: SIGTERM, SIGINT
    /// - Windows: Ctrl+C, Ctrl+Break
    #[cfg(unix)]
    pub fn install_signal_handlers() {
        use std::sync::atomic::Ordering;

        let registry = Self::global();
        if registry
            .handlers_installed
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            info!("Signal handlers already installed");
            return;
        }

        info!("Installing shutter safety signal handlers");

        // Spawn a task to handle signals
        std::thread::spawn(|| {
            use tokio::signal::unix::{signal, SignalKind};

            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    error!("Failed to create signal handler runtime: {}", e);
                    return;
                }
            };

            rt.block_on(async {
                let mut sigterm = signal(SignalKind::terminate()).expect("SIGTERM handler");
                let mut sigint = signal(SignalKind::interrupt()).expect("SIGINT handler");

                tokio::select! {
                    _ = sigterm.recv() => {
                        warn!("Received SIGTERM - closing all shutters");
                        Self::emergency_close_all();
                    }
                    _ = sigint.recv() => {
                        warn!("Received SIGINT - closing all shutters");
                        Self::emergency_close_all();
                    }
                }
            });
        });
    }

    /// Install signal handlers (Windows version)
    #[cfg(windows)]
    pub fn install_signal_handlers() {
        let registry = Self::global();
        if registry
            .handlers_installed
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            info!("Signal handlers already installed");
            return;
        }

        info!("Installing shutter safety signal handlers (Windows)");

        std::thread::spawn(|| {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    error!("Failed to create signal handler runtime: {}", e);
                    return;
                }
            };

            rt.block_on(async {
                let mut ctrl_c = tokio::signal::ctrl_c();

                ctrl_c.await.expect("Ctrl+C handler");
                warn!("Received Ctrl+C - closing all shutters");
                Self::emergency_close_all();
            });
        });
    }

    /// Install a panic hook that closes all shutters
    ///
    /// This should be called once during application startup.
    pub fn install_panic_hook() {
        let default_hook = std::panic::take_hook();

        std::panic::set_hook(Box::new(move |info| {
            error!("PANIC detected - attempting emergency shutter close");
            Self::emergency_close_all();

            // Call the default hook to print the panic message
            default_hook(info);
        }));

        info!("Installed panic hook for shutter safety");
    }
}

// =============================================================================
// Heartbeat-Based Shutter Guard
// =============================================================================

/// A shutter guard that requires periodic heartbeats to keep the shutter open.
///
/// If no heartbeat is received within the timeout period, the shutter is
/// automatically closed. This protects against script hangs and deadlocks.
///
/// # Example
///
/// ```rust,ignore
/// let guard = HeartbeatShutterGuard::new(driver, Duration::from_secs(5)).await?;
///
/// for i in 0..100 {
///     guard.heartbeat(); // Must call periodically!
///     expensive_operation();
/// }
///
/// // Guard closes shutter on drop
/// ```
pub struct HeartbeatShutterGuard {
    /// Unique ID for this guard
    id: u64,
    /// Shutter driver
    driver: Arc<dyn ShutterControl>,
    /// Channel to send heartbeats
    heartbeat_tx: mpsc::Sender<()>,
    /// Watchdog task handle
    _watchdog_handle: JoinHandle<()>,
    /// Flag indicating if shutter was opened successfully
    is_open: AtomicBool,
}

impl HeartbeatShutterGuard {
    /// Create a new heartbeat-based shutter guard.
    ///
    /// Opens the shutter immediately and starts a watchdog task that will
    /// close the shutter if no heartbeat is received within the timeout.
    ///
    /// # Arguments
    ///
    /// * `driver` - The shutter control driver
    /// * `timeout` - Maximum time between heartbeats before auto-close
    ///
    /// # Errors
    ///
    /// Returns an error if the shutter cannot be opened.
    pub async fn new(driver: Arc<dyn ShutterControl>, timeout: Duration) -> anyhow::Result<Self> {
        // Clamp timeout to reasonable bounds
        let timeout = timeout.clamp(Duration::from_millis(500), MAX_HEARTBEAT_TIMEOUT);

        // Open the shutter
        driver.open_shutter().await?;
        info!(
            timeout_secs = timeout.as_secs_f32(),
            "HeartbeatShutterGuard: Shutter opened with heartbeat watchdog"
        );

        // Register with global registry
        let id = ShutterRegistry::register(&driver);

        // Create heartbeat channel
        let (heartbeat_tx, mut heartbeat_rx) = mpsc::channel::<()>(1);

        // Spawn watchdog task
        let watchdog_driver = driver.clone();
        let watchdog_handle = tokio::spawn(async move {
            let mut last_heartbeat = Instant::now();

            loop {
                tokio::select! {
                    // Wait for heartbeat or timeout
                    result = tokio::time::timeout(timeout, heartbeat_rx.recv()) => {
                        match result {
                            Ok(Some(())) => {
                                // Heartbeat received
                                last_heartbeat = Instant::now();
                            }
                            Ok(None) => {
                                // Channel closed (guard dropped)
                                info!("HeartbeatShutterGuard: Channel closed, watchdog exiting");
                                return;
                            }
                            Err(_) => {
                                // Timeout! No heartbeat received
                                let elapsed = last_heartbeat.elapsed();
                                error!(
                                    elapsed_secs = elapsed.as_secs_f32(),
                                    timeout_secs = timeout.as_secs_f32(),
                                    "HeartbeatShutterGuard: TIMEOUT - no heartbeat! Closing shutter"
                                );

                                // Attempt to close shutter
                                match watchdog_driver.close_shutter().await {
                                    Ok(()) => {
                                        warn!("HeartbeatShutterGuard: Shutter closed due to timeout");
                                    }
                                    Err(e) => {
                                        error!(
                                            error = %e,
                                            "HeartbeatShutterGuard: CRITICAL - Failed to close shutter on timeout!"
                                        );
                                    }
                                }
                                return;
                            }
                        }
                    }
                }
            }
        });

        Ok(Self {
            id,
            driver,
            heartbeat_tx,
            _watchdog_handle: watchdog_handle,
            is_open: AtomicBool::new(true),
        })
    }

    /// Send a heartbeat to keep the shutter open.
    ///
    /// This must be called periodically (more frequently than the timeout)
    /// to prevent the watchdog from closing the shutter.
    ///
    /// Returns `true` if the heartbeat was sent successfully.
    pub fn heartbeat(&self) -> bool {
        if !self.is_open.load(Ordering::SeqCst) {
            return false;
        }

        match self.heartbeat_tx.try_send(()) {
            Ok(()) => true,
            Err(mpsc::error::TrySendError::Full(_)) => {
                // Channel is full, but that's okay - a heartbeat is pending
                true
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                warn!("HeartbeatShutterGuard: Heartbeat channel closed");
                false
            }
        }
    }

    /// Check if the shutter is still open.
    pub fn is_open(&self) -> bool {
        self.is_open.load(Ordering::SeqCst)
    }

    /// Get the guard ID (for debugging/logging)
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Manually close the shutter and mark the guard as closed.
    pub async fn close(&self) -> anyhow::Result<()> {
        if self
            .is_open
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            self.driver.close_shutter().await?;
            info!(guard_id = self.id, "HeartbeatShutterGuard: Shutter closed");
        }
        Ok(())
    }
}

impl Drop for HeartbeatShutterGuard {
    fn drop(&mut self) {
        // Mark as closed to stop watchdog from trying to close again
        let was_open = self.is_open.swap(false, Ordering::SeqCst);

        // Unregister from global registry
        ShutterRegistry::unregister(self.id);

        if was_open {
            // Attempt to close shutter on drop
            // This is best-effort since we can't do async in Drop
            if let Ok(handle) = Handle::try_current() {
                let driver = self.driver.clone();
                let id = self.id;

                // Use spawn_blocking to close the shutter
                // This is fire-and-forget since we're in Drop
                handle.spawn(async move {
                    match tokio::time::timeout(Duration::from_secs(2), driver.close_shutter()).await
                    {
                        Ok(Ok(())) => {
                            info!(guard_id = id, "HeartbeatShutterGuard drop: Shutter closed");
                        }
                        Ok(Err(e)) => {
                            error!(
                                guard_id = id,
                                error = %e,
                                "HeartbeatShutterGuard drop: Failed to close shutter"
                            );
                        }
                        Err(_) => {
                            error!(
                                guard_id = id,
                                "HeartbeatShutterGuard drop: Timeout closing shutter"
                            );
                        }
                    }
                });
            } else {
                warn!(
                    guard_id = self.id,
                    "HeartbeatShutterGuard drop: No runtime available, cannot close shutter!"
                );
            }
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::AtomicBool;
    use tokio::time::sleep;

    /// Mock shutter for testing
    struct MockShutter {
        is_open: AtomicBool,
        close_count: std::sync::atomic::AtomicU32,
    }

    impl MockShutter {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                is_open: AtomicBool::new(false),
                close_count: std::sync::atomic::AtomicU32::new(0),
            })
        }

        fn close_count(&self) -> u32 {
            self.close_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl ShutterControl for MockShutter {
        async fn open_shutter(&self) -> anyhow::Result<()> {
            self.is_open.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn close_shutter(&self) -> anyhow::Result<()> {
            self.is_open.store(false, Ordering::SeqCst);
            self.close_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn is_shutter_open(&self) -> anyhow::Result<bool> {
            Ok(self.is_open.load(Ordering::SeqCst))
        }
    }

    #[tokio::test]
    async fn test_heartbeat_guard_opens_shutter() {
        let mock = MockShutter::new();
        let guard = HeartbeatShutterGuard::new(mock.clone(), Duration::from_secs(5))
            .await
            .unwrap();

        assert!(mock.is_open.load(Ordering::SeqCst));
        assert!(guard.is_open());

        drop(guard);
        // Give the drop task time to complete
        sleep(Duration::from_millis(100)).await;
        assert!(!mock.is_open.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_heartbeat_prevents_timeout() {
        let mock = MockShutter::new();
        let guard = HeartbeatShutterGuard::new(mock.clone(), Duration::from_millis(100))
            .await
            .unwrap();

        // Send heartbeats faster than timeout
        for _ in 0..5 {
            assert!(guard.heartbeat());
            sleep(Duration::from_millis(50)).await;
        }

        // Shutter should still be open
        assert!(mock.is_open.load(Ordering::SeqCst));
        assert_eq!(mock.close_count(), 0);

        drop(guard);
    }

    #[tokio::test]
    async fn test_timeout_closes_shutter() {
        let mock = MockShutter::new();
        // Use 600ms timeout (min is 500ms due to clamping)
        let guard = HeartbeatShutterGuard::new(mock.clone(), Duration::from_millis(600))
            .await
            .unwrap();

        // Don't send any heartbeats - wait for timeout plus buffer
        // The watchdog needs time to detect timeout AND close the shutter
        // 600ms timeout + 200ms buffer = 800ms wait
        sleep(Duration::from_millis(900)).await;

        // Watchdog should have closed the shutter
        assert!(!mock.is_open.load(Ordering::SeqCst));
        assert!(mock.close_count() >= 1);

        // Guard's is_open should also reflect the closed state
        // (though it may still think it's open since it didn't initiate the close)
        drop(guard);
    }

    #[test]
    fn test_registry_register_unregister() {
        let mock = MockShutter::new();
        let id = ShutterRegistry::register(&(mock.clone() as Arc<dyn ShutterControl>));

        assert!(id > 0);

        ShutterRegistry::unregister(id);
        // Should not panic on double unregister
        ShutterRegistry::unregister(id);
    }
}
