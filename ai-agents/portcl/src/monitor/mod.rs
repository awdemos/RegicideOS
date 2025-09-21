pub mod portage;
pub mod metrics;
pub mod events;

pub use portage::{PortageMonitor, PortageInfo, PackageInfo};
pub use metrics::{PortageMetrics, SystemMetrics, MetricsCollector};
pub use events::{PortageEvent, EventTracker, EventType};

use crate::config::MonitoringConfig;
use crate::error::Result;
use std::sync::{Arc, Mutex};
use tracing::{info, debug, error};

pub struct MonitorManager {
    config: MonitoringConfig,
    portage_monitor: PortageMonitor,
    metrics_collector: Arc<Mutex<MetricsCollector>>,
    event_tracker: EventTracker,
}

impl MonitorManager {
    pub fn new(config: MonitoringConfig) -> Result<Self> {
        let portage_monitor = PortageMonitor::new(config.clone())?;
        let metrics_collector = Arc::new(Mutex::new(MetricsCollector::new(config.clone())?));
        let event_tracker = EventTracker::new(config.clone())?;

        Ok(Self {
            config,
            portage_monitor,
            metrics_collector,
            event_tracker,
        })
    }

    pub async fn collect_metrics(&self) -> Result<PortageMetrics> {
        debug!("Starting metrics collection");

        let portage_info = self.portage_monitor.get_portage_info().await?;
        let mut metrics_collector = self.metrics_collector.lock().unwrap();
        let system_metrics = metrics_collector.collect_system_metrics().await?;
        drop(metrics_collector);

        let metrics = PortageMetrics {
            timestamp: chrono::Utc::now(),
            portage_info,
            system_metrics,
            recent_events: self.event_tracker.get_recent_events(Some(100)).await,
        };

        debug!("Metrics collection completed");
        Ok(metrics)
    }

    pub async fn start_monitoring(&self) -> Result<()> {
        info!("Starting Portage monitoring");

        let interval = self.config.poll_interval;
        let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(interval));

        loop {
            ticker.tick().await;

            match self.collect_metrics().await {
                Ok(metrics) => {
                    debug!("Collected metrics: {} packages, {:.1}% CPU load",
                           metrics.portage_info.installed_packages,
                           metrics.system_metrics.cpu_usage_percent);

                    // Store metrics for later analysis
                    // TODO: Implement metrics storage
                }
                Err(e) => {
                    error!("Failed to collect metrics: {}", e);
                }
            }
        }
    }

    pub async fn get_package_info(&self, package: &str) -> Result<PackageInfo> {
        self.portage_monitor.get_package_info(package).await
    }

    pub async fn track_event(&self, event_type: EventType, details: String) -> Result<()> {
        self.event_tracker.track_event(event_type, details).await.map(|_| ())
    }
}