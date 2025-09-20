use crate::config::RLConfig;
use crate::error::{PortCLError, Result};
use crate::monitor::PortageMetrics;
use crate::actions::Action;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use ndarray::{Array1, Array2, Array3};
use rand::Rng;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub input_size: usize,
    pub hidden_sizes: Vec<usize>,
    pub output_size: usize,
    pub learning_rate: f64,
    pub dropout_rate: f64,
    pub batch_norm: bool,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            input_size: 50,  // State vector size
            hidden_sizes: vec![128, 128, 64],  // 3 hidden layers
            output_size: 6,  // Number of possible actions
            learning_rate: 0.001,
            dropout_rate: 0.1,
            batch_norm: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DQNModel {
    pub config: ModelConfig,
    pub model_type: ModelType,
    // Note: tch::nn::Module would be stored here when libtorch is available
    pub training_step: u64,
    pub target_update_freq: usize,
    pub epsilon: f64,
    pub epsilon_min: f64,
    pub epsilon_decay: f64,
    pub loss_history: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelType {
    Online,
    Target,
}

impl DQNModel {
    pub fn new(config: ModelConfig, target_update_freq: usize) -> Result<Self> {
        Ok(Self {
            config,
            model_type: ModelType::Online,
            training_step: 0,
            target_update_freq,
            epsilon: 0.1,
            epsilon_min: 0.01,
            epsilon_decay: 0.995,
            loss_history: Vec::new(),
        })
    }

    pub fn create_target_model(&self) -> Result<DQNModel> {
        let mut target = self.clone();
        target.model_type = ModelType::Target;
        target.training_step = 0;
        Ok(target)
    }

    pub fn state_to_tensor(&self, metrics: &PortageMetrics) -> Result<Array1<f64>> {
        let mut state = Array1::zeros(self.config.input_size);

        // Convert PortageMetrics to state vector
        let mut idx = 0;

        // Portage information
        state[idx] = metrics.portage_info.installed_packages as f64 / 1000.0;  // Normalize
        idx += 1;
        state[idx] = metrics.portage_info.available_updates as f64 / 100.0;  // Normalize
        idx += 1;
        state[idx] = metrics.portage_info.world_packages as f64 / 100.0;  // Normalize
        idx += 1;

        // System metrics
        state[idx] = metrics.system_metrics.cpu_usage_percent / 100.0;  // Normalize to 0-1
        idx += 1;
        state[idx] = metrics.system_metrics.memory_usage_percent / 100.0;  // Normalize
        idx += 1;
        state[idx] = metrics.system_metrics.disk_usage_percent / 100.0;  // Normalize
        idx += 1;
        state[idx] = metrics.system_metrics.load_average_1min / 10.0;  // Normalize
        idx += 1;
        state[idx] = metrics.system_metrics.load_average_5min / 10.0;  // Normalize
        idx += 1;
        state[idx] = metrics.system_metrics.load_average_15min / 10.0;  // Normalize
        idx += 1;

        // Process count (normalized)
        state[idx] = metrics.system_metrics.process_count as f64 / 1000.0;
        idx += 1;

        // Uptime (normalized to days)
        state[idx] = metrics.system_metrics.uptime_seconds as f64 / (24.0 * 3600.0);
        idx += 1;

        // Temperature (if available, normalized)
        if let Some(temp) = metrics.system_metrics.temperature_celsius {
            state[idx] = (temp - 20.0) / 60.0;  // Normalize assuming 20-80°C range
        }
        idx += 1;

        // Network I/O (normalized)
        state[idx] = (metrics.system_metrics.network_io.bytes_received as f64 / (1024.0 * 1024.0 * 1024.0)).min(1.0);
        idx += 1;
        state[idx] = (metrics.system_metrics.network_io.bytes_transmitted as f64 / (1024.0 * 1024.0 * 1024.0)).min(1.0);
        idx += 1;

        // Recent events summary (last 10 events)
        let recent_events = &metrics.recent_events;
        let event_summary = self.summarize_events(recent_events);
        for event_val in event_summary {
            state[idx] = event_val;
            idx += 1;
        }

        // Fill remaining with zeros if needed
        while idx < self.config.input_size {
            state[idx] = 0.0;
            idx += 1;
        }

        Ok(state)
    }

    fn summarize_events(&self, events: &[crate::monitor::events::PortageEvent]) -> Vec<f64> {
        let mut summary = vec![0.0; 20];  // Allocate 20 slots for event summary

        // Count different event types
        let mut install_count = 0;
        let mut update_count = 0;
        let mut remove_count = 0;
        let mut compile_success = 0;
        let mut compile_failed = 0;
        let mut sync_success = 0;
        let mut sync_failed = 0;
        let mut error_count = 0;
        let mut warning_count = 0;

        for event in events.iter().take(10) {  // Last 10 events
            match event.event_type {
                crate::monitor::events::EventType::PackageInstall => install_count += 1,
                crate::monitor::events::EventType::PackageUpdate => update_count += 1,
                crate::monitor::events::EventType::PackageRemove => remove_count += 1,
                crate::monitor::events::EventType::CompileSuccess => compile_success += 1,
                crate::monitor::events::EventType::CompileFailed => compile_failed += 1,
                crate::monitor::events::EventType::SyncComplete => sync_success += 1,
                crate::monitor::events::EventType::SyncFailed => sync_failed += 1,
                crate::monitor::events::EventType::Error => error_count += 1,
                crate::monitor::events::EventType::Warning => warning_count += 1,
                _ => {}
            }
        }

        // Normalize counts
        summary[0] = install_count as f64 / 10.0;
        summary[1] = update_count as f64 / 10.0;
        summary[2] = remove_count as f64 / 10.0;
        summary[3] = compile_success as f64 / 10.0;
        summary[4] = compile_failed as f64 / 10.0;
        summary[5] = sync_success as f64 / 10.0;
        summary[6] = sync_failed as f64 / 10.0;
        summary[7] = error_count as f64 / 10.0;
        summary[8] = warning_count as f64 / 10.0;

        // Calculate rates
        let total_events = events.len().max(1);
        summary[9] = (compile_success as f64 / (compile_success + compile_failed).max(1) as f64) * 2.0 - 1.0;  // -1 to 1
        summary[10] = (sync_success as f64 / (sync_success + sync_failed).max(1) as f64) * 2.0 - 1.0;  // -1 to 1

        // Time since last event (normalized)
        if let Some(last_event) = events.first() {
            let duration = chrono::Utc::now().signed_duration_since(last_event.timestamp);
            summary[11] = (duration.num_minutes() as f64 / 60.0).min(1.0);  // Normalize to hours
        }

        summary
    }

    pub fn predict(&self, state: &Array1<f64>) -> Result<Array1<f64>> {
        // Placeholder implementation - would use tch::Tensor when libtorch is available
        debug!("Predicting Q-values for state");

        // Simple linear model as fallback
        let mut q_values = Array1::zeros(self.config.output_size);

        // Heuristic-based Q-values for now
        q_values[0] = 0.0;  // NoOp
        q_values[1] = self.calculate_parallelism_q_value(state);  // Adjust parallelism
        q_values[2] = self.calculate_build_order_q_value(state);  // Optimize build order
        q_values[3] = self.calculate_schedule_q_value(state);  // Schedule operation
        q_values[4] = self.calculate_prefetch_q_value(state);  // Pre-fetch dependencies
        q_values[5] = self.calculate_cleanup_q_value(state);  // Clean obsolete packages

        // Add exploration noise
        if self.should_explore() {
            let mut rng = rand::thread_rng();
            for i in 0..q_values.len() {
                q_values[i] += rng.gen_range(-0.1..0.1);
            }
        }

        Ok(q_values)
    }

    fn calculate_parallelism_q_value(&self, state: &Array1<f64>) -> f64 {
        let cpu_usage = state[3];  // Normalized CPU usage
        let memory_usage = state[4];  // Normalized memory usage
        let load_avg = state[6];  // Normalized 1-min load average

        // Favor parallelism adjustment when system is underutilized
        if cpu_usage < 0.7 && memory_usage < 0.8 && load_avg < 0.8 {
            0.5
        } else {
            -0.2
        }
    }

    fn calculate_build_order_q_value(&self, state: &Array1<f64>) -> f64 {
        let available_updates = state[1];  // Normalized available updates

        // Favor build order optimization when there are many updates
        if available_updates > 0.3 {
            0.3
        } else {
            0.0
        }
    }

    fn calculate_schedule_q_value(&self, state: &Array1<f64>) -> f64 {
        let cpu_usage = state[3];  // Normalized CPU usage
        let load_avg = state[6];  // Normalized 1-min load average

        // Favor scheduling when system is busy
        if cpu_usage > 0.8 || load_avg > 0.8 {
            0.4
        } else {
            -0.1
        }
    }

    fn calculate_prefetch_q_value(&self, state: &Array1<f64>) -> f64 {
        let network_activity = state[13] + state[14];  // Combined network I/O

        // Favor pre-fetching when network is idle
        if network_activity < 0.3 {
            0.2
        } else {
            -0.1
        }
    }

    fn calculate_cleanup_q_value(&self, state: &Array1<f64>) -> f64 {
        let disk_usage = state[5];  // Normalized disk usage

        // Favor cleanup when disk is getting full
        if disk_usage > 0.8 {
            0.6
        } else {
            -0.2
        }
    }

    pub fn select_action(&mut self, state: &Array1<f64>) -> Result<(Action, f64)> {
        let q_values = self.predict(state)?;

        // Epsilon-greedy action selection
        if self.should_explore() {
            let mut rng = rand::thread_rng();
            let action_idx = rng.gen_range(0..q_values.len());
            let action = Self::index_to_action(action_idx)?;
            return Ok((action, q_values[action_idx]));
        }

        // Select action with highest Q-value
        let max_q = q_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let action_idx = q_values.iter().position(|&x| x == max_q)
            .ok_or_else(|| PortCLError::RLEngine("No valid action found".to_string()))?;

        let action = Self::index_to_action(action_idx)?;
        Ok((action, max_q))
    }

    fn should_explore(&self) -> bool {
        let mut rng = rand::thread_rng();
        rng.gen::<f64>() < self.epsilon
    }

    fn index_to_action(index: usize) -> Result<Action> {
        match index {
            0 => Ok(Action::NoOp),
            1 => Ok(Action::AdjustParallelism { jobs: 1 }),
            2 => Ok(Action::OptimizeBuildOrder { package_list: Vec::new() }),
            3 => Ok(Action::ScheduleOperation { delay_seconds: 0 }),
            4 => Ok(Action::PreFetchDependencies { packages: Vec::new() }),
            5 => Ok(Action::CleanObsoletePackages { force: false }),
            _ => Err(PortCLError::RLEngine(format!("Invalid action index: {}", index))),
        }
    }

    pub fn update_epsilon(&mut self) {
        self.epsilon = (self.epsilon * self.epsilon_decay).max(self.epsilon_min);
    }

    pub fn train(&mut self, _batch: &[Experience]) -> Result<f64> {
        // Placeholder implementation - would use tch when available
        debug!("Training model with batch of size {}", _batch.len());

        // Simulate training loss
        let mut rng = rand::thread_rng();
        let loss = rng.gen_range(0.01..0.1);

        self.loss_history.push(loss);
        self.training_step += 1;

        Ok(loss)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        // Save model configuration and training state
        let model_data = serde_json::to_string_pretty(self)
            .map_err(|e| PortCLError::Json(e))?;

        std::fs::write(path, model_data)
            .map_err(|e| PortCLError::Io(e))?;

        info!("Model saved to {}", path.display());
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let model_data = std::fs::read_to_string(path)
            .map_err(|e| PortCLError::Io(e))?;

        let model: Self = serde_json::from_str(&model_data)
            .map_err(|e| PortCLError::Json(e))?;

        info!("Model loaded from {}", path.display());
        Ok(model)
    }

    pub fn update_target_network(&mut self, target_model: &DQNModel) -> Result<()> {
        // In real implementation, this would copy neural network weights
        debug!("Updating target network");

        // For now, just update training step
        target_model.training_step = self.training_step;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub state: Array1<f64>,
    pub action: Action,
    pub reward: f64,
    pub next_state: Array1<f64>,
    pub done: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Experience {
    pub fn new(
        state: Array1<f64>,
        action: Action,
        reward: f64,
        next_state: Array1<f64>,
        done: bool,
    ) -> Self {
        Self {
            state,
            action,
            reward,
            next_state,
            done,
            timestamp: chrono::Utc::now(),
        }
    }
}