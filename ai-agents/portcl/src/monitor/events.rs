use crate::config::MonitoringConfig;
use crate::error::{PortCLError, Result};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::{VecDeque, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use uuid::Uuid;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortageEvent {
    pub id: String,
    pub event_type: EventType,
    pub timestamp: DateTime<Utc>,
    pub details: String,
    pub severity: EventSeverity,
    pub source: EventSource,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EventType {
    PackageInstall,
    PackageRemove,
    PackageUpdate,
    SyncStart,
    SyncComplete,
    SyncFailed,
    CompileStart,
    CompileSuccess,
    CompileFailed,
    DependencyResolution,
    ConfigurationChange,
    SystemStateChange,
    UserAction,
    AgentAction,
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EventSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EventSource {
    Portage,
    System,
    User,
    Agent,
    External,
}

pub struct EventTracker {
    config: MonitoringConfig,
    events: Arc<RwLock<VecDeque<PortageEvent>>>,
    max_events: usize,
}

impl EventTracker {
    pub fn new(config: MonitoringConfig) -> Result<Self> {
        Ok(Self {
            config,
            events: Arc::new(RwLock::new(VecDeque::new())),
            max_events: 1000, // Keep last 1000 events in memory
        })
    }

    pub async fn track_event(&self, event_type: EventType, details: String) -> Result<String> {
        let event = PortageEvent {
            id: Uuid::new_v4().to_string(),
            event_type: event_type.clone(),
            timestamp: Utc::now(),
            details,
            severity: self.determine_severity(&event_type),
            source: self.determine_source(&event_type),
            metadata: None,
        };

        debug!("Tracking event: {:?} - {}", event.event_type, event.details);

        let mut events = self.events.write().await;
        events.push_back(event.clone());

        // Maintain event limit
        if events.len() > self.max_events {
            events.pop_front();
        }

        // Log to file if enabled
        if self.config.enable_event_tracking {
            self.log_to_file(&event).await?;
        }

        Ok(event.id)
    }

    pub async fn get_recent_events(&self, limit: Option<usize>) -> Vec<PortageEvent> {
        let events = self.events.read().await;
        let limit = limit.unwrap_or(100);

        events.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub async fn get_events_by_type(&self, event_type: EventType) -> Vec<PortageEvent> {
        let events = self.events.read().await;

        events.iter()
            .filter(|event| event.event_type == event_type)
            .cloned()
            .collect()
    }

    pub async fn get_events_by_severity(&self, severity: EventSeverity) -> Vec<PortageEvent> {
        let events = self.events.read().await;

        events.iter()
            .filter(|event| event.severity == severity)
            .cloned()
            .collect()
    }

    pub async fn get_events_in_timerange(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>
    ) -> Vec<PortageEvent> {
        let events = self.events.read().await;

        events.iter()
            .filter(|event| event.timestamp >= start && event.timestamp <= end)
            .cloned()
            .collect()
    }

    pub async fn get_error_events(&self) -> Vec<PortageEvent> {
        self.get_events_by_type(EventType::Error).await
    }

    pub async fn get_compilation_events(&self) -> Vec<PortageEvent> {
        let mut events = Vec::new();
        let all_events = self.events.read().await;

        for event in all_events.iter() {
            match event.event_type {
                EventType::CompileStart |
                EventType::CompileSuccess |
                EventType::CompileFailed => {
                    events.push(event.clone());
                }
                _ => {}
            }
        }

        events
    }

    pub async fn get_package_events(&self, package_name: &str) -> Vec<PortageEvent> {
        let events = self.events.read().await;

        events.iter()
            .filter(|event| {
                event.details.contains(package_name) ||
                (event.event_type == EventType::PackageInstall && event.details.contains(package_name)) ||
                (event.event_type == EventType::PackageUpdate && event.details.contains(package_name)) ||
                (event.event_type == EventType::PackageRemove && event.details.contains(package_name))
            })
            .cloned()
            .collect()
    }

    pub async fn get_event_statistics(&self) -> EventStatistics {
        let events = self.events.read().await;

        let mut stats = EventStatistics {
            total_events: events.len(),
            events_by_type: std::collections::HashMap::new(),
            events_by_severity: std::collections::HashMap::new(),
            events_by_source: std::collections::HashMap::new(),
            recent_events: events.iter().rev().take(10).cloned().collect(),
        };

        for event in events.iter() {
            *stats.events_by_type.entry(event.event_type.clone()).or_insert(0) += 1;
            *stats.events_by_severity.entry(event.severity.clone()).or_insert(0) += 1;
            *stats.events_by_source.entry(event.source.clone()).or_insert(0) += 1;
        }

        stats
    }

    async fn log_to_file(&self, event: &PortageEvent) -> Result<()> {
        use std::fs::OpenOptions;
        use std::io::Write;

        let log_file = &self.config.log_path;

        // Create directory if it doesn't exist
        if let Some(parent) = std::path::Path::new(log_file).parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| PortCLError::Io(e))?;
        }

        // Serialize event to JSON
        let event_json = serde_json::to_string(event)
            .map_err(|e| PortCLError::Json(e))?;

        // Append to log file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
            .map_err(|e| PortCLError::Io(e))?;

        writeln!(file, "{}", event_json)
            .map_err(|e| PortCLError::Io(e))?;

        Ok(())
    }

    fn determine_severity(&self, event_type: &EventType) -> EventSeverity {
        match event_type {
            EventType::Error => EventSeverity::High,
            EventType::CompileFailed => EventSeverity::Medium,
            EventType::SyncFailed => EventSeverity::Medium,
            EventType::Warning => EventSeverity::Medium,
            EventType::PackageInstall |
            EventType::PackageRemove |
            EventType::PackageUpdate => EventSeverity::Low,
            EventType::CompileStart |
            EventType::CompileSuccess |
            EventType::SyncStart |
            EventType::SyncComplete |
            EventType::DependencyResolution |
            EventType::ConfigurationChange |
            EventType::SystemStateChange => EventSeverity::Info,
            EventType::UserAction |
            EventType::AgentAction => EventSeverity::Low,
            EventType::Info => EventSeverity::Info,
        }
    }

    fn determine_source(&self, event_type: &EventType) -> EventSource {
        match event_type {
            EventType::PackageInstall |
            EventType::PackageRemove |
            EventType::PackageUpdate |
            EventType::SyncStart |
            EventType::SyncComplete |
            EventType::SyncFailed |
            EventType::CompileStart |
            EventType::CompileSuccess |
            EventType::CompileFailed |
            EventType::DependencyResolution => EventSource::Portage,
            EventType::ConfigurationChange |
            EventType::SystemStateChange => EventSource::System,
            EventType::UserAction => EventSource::User,
            EventType::AgentAction => EventSource::Agent,
            EventType::Error |
            EventType::Warning |
            EventType::Info => EventSource::External,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStatistics {
    pub total_events: usize,
    pub events_by_type: std::collections::HashMap<EventType, usize>,
    pub events_by_severity: std::collections::HashMap<EventSeverity, usize>,
    pub events_by_source: std::collections::HashMap<EventSource, usize>,
    pub recent_events: Vec<PortageEvent>,
}

impl EventStatistics {
    pub fn get_error_rate(&self) -> f64 {
        let total_events = self.total_events as f64;
        if total_events == 0.0 {
            return 0.0;
        }

        let error_count = self.events_by_type.get(&EventType::Error).unwrap_or(&0) as f64;
        (error_count / total_events) * 100.0
    }

    pub fn get_compilation_success_rate(&self) -> f64 {
        let success_count = self.events_by_type.get(&EventType::CompileSuccess).unwrap_or(&0);
        let failed_count = self.events_by_type.get(&EventType::CompileFailed).unwrap_or(&0);
        let total = success_count + failed_count;

        if total == 0 {
            return 100.0;
        }

        (*success_count as f64 / total as f64) * 100.0
    }

    pub fn get_sync_success_rate(&self) -> f64 {
        let success_count = self.events_by_type.get(&EventType::SyncComplete).unwrap_or(&0);
        let failed_count = self.events_by_type.get(&EventType::SyncFailed).unwrap_or(&0);
        let total = success_count + failed_count;

        if total == 0 {
            return 100.0;
        }

        (*success_count as f64 / total as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_tracking() {
        let config = MonitoringConfig::default();
        let tracker = EventTracker::new(config).unwrap();

        let event_id = tracker.track_event(EventType::Info, "Test event".to_string()).await.unwrap();

        let events = tracker.get_recent_events(Some(1)).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, event_id);
        assert_eq!(events[0].details, "Test event");
    }

    #[tokio::test]
    async fn test_event_statistics() {
        let config = MonitoringConfig::default();
        let tracker = EventTracker::new(config).unwrap();

        // Track some test events
        tracker.track_event(EventType::CompileSuccess, "Package compiled".to_string()).await.unwrap();
        tracker.track_event(EventType::CompileFailed, "Package failed".to_string()).await.unwrap();
        tracker.track_event(EventType::SyncComplete, "Sync successful".to_string()).await.unwrap();

        let stats = tracker.get_event_statistics().await;
        assert_eq!(stats.total_events, 3);
        assert_eq!(stats.get_compilation_success_rate(), 50.0);
        assert_eq!(stats.get_sync_success_rate(), 100.0);
    }
}