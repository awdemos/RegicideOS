use crate::config::RLConfig;
use crate::error::{PortCLError, Result};
use crate::monitor::PortageMetrics;
use crate::actions::Action;
use crate::rl_engine::{
    model::{DQNModel, ModelConfig, Experience},
    experience::ReplayBuffer,
    continual::{ContinualLearning, ContinualLearningConfig, TaskContext},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use ndarray::Array1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub model_config: ModelConfig,
    pub buffer_capacity: usize,
    pub batch_size: usize,
    pub target_update_freq: usize,
    pub training_freq: usize,
    pub save_freq: usize,
    pub model_path: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model_config: ModelConfig::default(),
            buffer_capacity: 10000,
            batch_size: 32,
            target_update_freq: 100,
            training_freq: 10,
            save_freq: 1000,
            model_path: "/var/lib/portcl/model.pt".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub training_step: u64,
    pub episodes_completed: u64,
    pub total_reward: f64,
    pub average_reward: f64,
    pub success_count: u64,
    pub failure_count: u64,
    pub current_episode_steps: u64,
    pub last_training_loss: f64,
    pub epsilon: f64,
    pub learning_active: bool,
}

pub struct PortageAgent {
    pub config: AgentConfig,
    pub model: Arc<RwLock<DQNModel>>,
    pub target_model: Arc<RwLock<DQNModel>>,
    pub replay_buffer: ReplayBuffer,
    pub continual_learning: ContinualLearning,
    pub state: Arc<Mutex<AgentState>>,
    pub reward_calculator: RewardCalculator,
    pub task_tracker: TaskTracker,
}

impl PortageAgent {
    pub fn new(rl_config: RLConfig) -> Result<Self> {
        let agent_config = AgentConfig {
            model_config: ModelConfig::default(),
            buffer_capacity: rl_config.memory_size,
            batch_size: rl_config.batch_size,
            target_update_freq: rl_config.target_update_freq,
            training_freq: 10,
            save_freq: 1000,
            model_path: rl_config.model_path.to_string_lossy().to_string(),
        };

        let model = DQNModel::new(agent_config.model_config.clone(), agent_config.target_update_freq)?;
        let target_model = model.create_target_model()?;

        let replay_buffer = ReplayBuffer::new(crate::rl_engine::experience::ReplayBufferConfig {
            capacity: agent_config.buffer_capacity,
            use_prioritized_replay: true,
            alpha: 0.6,
            beta_start: 0.4,
            beta_increment: 0.001,
        });

        let continual_learning = ContinualLearning::new(ContinualLearningConfig {
            enable_ewc: rl_config.enable_continual_learning,
            ewc_importance: rl_config.ewc_importance,
            enable_progressive_networks: true,
            enable_policy_reuse: true,
            consolidation_threshold: 0.1,
            memory_retention_rate: 0.95,
            max_policies: 10,
            consolidation_interval: 100,
        })?;

        let state = AgentState {
            training_step: 0,
            episodes_completed: 0,
            total_reward: 0.0,
            average_reward: 0.0,
            success_count: 0,
            failure_count: 0,
            current_episode_steps: 0,
            last_training_loss: 0.0,
            epsilon: rl_config.exploration_rate,
            learning_active: true,
        };

        Ok(Self {
            config: agent_config,
            model: Arc::new(RwLock::new(model)),
            target_model: Arc::new(RwLock::new(target_model)),
            replay_buffer,
            continual_learning,
            state: Arc::new(Mutex::new(state)),
            reward_calculator: RewardCalculator::new(),
            task_tracker: TaskTracker::new(),
        })
    }

    pub async fn select_action(&self, metrics: &PortageMetrics) -> Result<Action> {
        debug!("Selecting action based on current system metrics");

        let state = self.metrics_to_state(metrics).await?;

        let mut model = self.model.write().await;
        let (action, q_value) = model.select_action(&state)?;

        debug!("Selected action: {} (Q-value: {:.3})", action.description(), q_value);

        Ok(action)
    }

    pub async fn update_experience(&mut self, experience: Experience) -> Result<()> {
        debug!("Updating agent experience");

        // Add experience to replay buffer
        let priority = self.calculate_priority(&experience).await;
        self.replay_buffer.add_experience(experience.clone(), Some(priority))?;

        // Update agent state
        let mut state = self.state.lock().await;
        state.current_episode_steps += 1;
        state.total_reward += experience.reward;

        // Track task progress
        self.task_tracker.update(experience.clone()).await?;

        // Perform training if conditions are met
        if state.training_step % self.config.training_freq as u64 == 0 &&
           self.replay_buffer.len() >= self.config.batch_size {
            self.train_step().await?;
        }

        // Update target network if needed
        if state.training_step % self.config.target_update_freq as u64 == 0 {
            self.update_target_network().await?;
        }

        // Save model if needed
        if state.training_step % self.config.save_freq as u64 == 0 {
            self.save_model().await?;
        }

        // Consolidate knowledge periodically
        if state.training_step % 500 == 0 {
            self.continual_learning.consolidate_knowledge().await?;
        }

        // Update exploration rate
        let mut model = self.model.write().await;
        model.update_epsilon();
        state.epsilon = model.epsilon;

        state.training_step += 1;

        Ok(())
    }

    pub async fn train_step(&self) -> Result<f64> {
        debug!("Performing training step");

        let (batch, importance_weights, indices) = self.replay_buffer.sample_batch(self.config.batch_size)?;

        let loss = self.train_on_batch(&batch, &importance_weights).await?;

        // Update priorities for prioritized experience replay
        let td_errors: Vec<f64> = futures::future::join_all(
            batch.iter().map(|exp| async move {
                self.calculate_td_error(exp).await.unwrap_or_else(|_| 0.0)
            })
        ).await;

        self.replay_buffer.update_priorities(&indices, &td_errors)?;

        // Update agent state
        let mut state = self.state.lock().await;
        state.last_training_loss = loss;

        debug!("Training step completed. Loss: {:.6}", loss);
        Ok(loss)
    }

    async fn train_on_batch(&self, batch: &[Experience], _importance_weights: &[f64]) -> Result<f64> {
        let mut model = self.model.write().await;
        model.train(batch)
    }

    async fn update_target_network(&self) -> Result<()> {
        debug!("Updating target network");

        let mut target_model = self.target_model.write().await;
        let mut model = self.model.write().await;

        target_model.update_target_network(&mut model)?;
        drop(target_model);

        info!("Target network updated");
        Ok(())
    }

    pub async fn save_model(&self) -> Result<()> {
        debug!("Saving model");

        let model = self.model.read().await;
        let path = std::path::Path::new(&self.config.model_path);

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| PortCLError::Io(e))?;
        }

        model.save(path)?;
        info!("Model saved to {}", self.config.model_path);
        Ok(())
    }

    pub async fn load_model(&self) -> Result<()> {
        debug!("Loading model");

        let path = std::path::Path::new(&self.config.model_path);
        if !path.exists() {
            warn!("Model file not found: {}", path.display());
            return Ok(());
        }

        let mut model = self.model.write().await;
        *model = DQNModel::load(path)?;

        info!("Model loaded from {}", self.config.model_path);
        Ok(())
    }

    async fn metrics_to_state(&self, metrics: &PortageMetrics) -> Result<Array1<f64>> {
        let model = self.model.read().await;
        model.state_to_tensor(metrics)
    }

    async fn calculate_priority(&self, experience: &Experience) -> f64 {
        // Calculate TD error as priority
        self.calculate_td_error(experience).await.unwrap_or(1.0).abs()
    }

    async fn calculate_td_error(&self, experience: &Experience) -> Result<f64> {
        // Simplified TD error calculation
        // In full implementation, this would use the neural network
        let current_q = self.estimate_q_value(&experience.state, &experience.action).await?;
        let max_next_q = self.estimate_max_q_value(&experience.next_state).await?;

        let target = experience.reward + if experience.done { 0.0 } else { 0.95 * max_next_q };
        let td_error = target - current_q;

        Ok(td_error)
    }

    async fn estimate_q_value(&self, state: &Array1<f64>, action: &Action) -> Result<f64> {
        // Simplified Q-value estimation
        // In full implementation, this would use the neural network
        let model = self.model.write().await;
        let q_values = model.predict(state)?;

        let action_idx = self.action_to_index(action)?;
        Ok(q_values[action_idx])
    }

    async fn estimate_max_q_value(&self, state: &Array1<f64>) -> Result<f64> {
        let model = self.model.write().await;
        let q_values = model.predict(state)?;
        Ok(q_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)))
    }

    fn action_to_index(&self, action: &Action) -> Result<usize> {
        match action {
            Action::NoOp => Ok(0),
            Action::AdjustParallelism { .. } => Ok(1),
            Action::OptimizeBuildOrder { .. } => Ok(2),
            Action::ScheduleOperation { .. } => Ok(3),
            Action::PreFetchDependencies { .. } => Ok(4),
            Action::CleanObsoletePackages { .. } => Ok(5),
        }
    }

    pub async fn get_agent_statistics(&self) -> AgentStatistics {
        let state = self.state.lock().await;
        let buffer_stats = self.replay_buffer.get_statistics().unwrap_or_default();
        let knowledge_insights = self.continual_learning.get_knowledge_insights().await;

        AgentStatistics {
            training_step: state.training_step,
            episodes_completed: state.episodes_completed,
            total_reward: state.total_reward,
            average_reward: state.average_reward,
            success_rate: if state.success_count + state.failure_count > 0 {
                state.success_count as f64 / (state.success_count + state.failure_count) as f64
            } else {
                0.0
            },
            epsilon: state.epsilon,
            buffer_size: buffer_stats.size,
            buffer_capacity: buffer_stats.capacity,
            last_training_loss: state.last_training_loss,
            learning_active: state.learning_active,
            total_policies: knowledge_insights.total_policies,
            knowledge_coverage: knowledge_insights.knowledge_coverage,
            catastrophic_forgetting_risk: knowledge_insights.catastrophic_forgetting_risk,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatistics {
    pub training_step: u64,
    pub episodes_completed: u64,
    pub total_reward: f64,
    pub average_reward: f64,
    pub success_rate: f64,
    pub epsilon: f64,
    pub buffer_size: usize,
    pub buffer_capacity: usize,
    pub last_training_loss: f64,
    pub learning_active: bool,
    pub total_policies: usize,
    pub knowledge_coverage: f64,
    pub catastrophic_forgetting_risk: f64,
}

pub struct RewardCalculator {
    reward_weights: HashMap<String, f64>,
}

impl RewardCalculator {
    pub fn new() -> Self {
        let mut weights = HashMap::new();
        weights.insert("performance_improvement".to_string(), 5.0);
        weights.insert("resource_efficiency".to_string(), 3.0);
        weights.insert("system_stability".to_string(), 4.0);
        weights.insert("time_saved".to_string(), 2.0);
        weights.insert("error_prevention".to_string(), 6.0);

        Self { reward_weights: weights }
    }

    pub fn calculate_reward(&self, prev_metrics: &PortageMetrics, curr_metrics: &PortageMetrics, action: &Action) -> f64 {
        let mut reward = 0.0;

        // Performance improvement
        let perf_improvement = self.calculate_performance_improvement(prev_metrics, curr_metrics);
        reward += perf_improvement * self.reward_weights["performance_improvement"];

        // Resource efficiency
        let resource_efficiency = self.calculate_resource_efficiency(prev_metrics, curr_metrics);
        reward += resource_efficiency * self.reward_weights["resource_efficiency"];

        // System stability
        let stability = self.calculate_system_stability(prev_metrics, curr_metrics);
        reward += stability * self.reward_weights["system_stability"];

        // Time saved (for specific actions)
        let time_saved = self.calculate_time_saved(prev_metrics, curr_metrics, action);
        reward += time_saved * self.reward_weights["time_saved"];

        // Action-specific bonuses/penalties
        let action_bonus = self.calculate_action_bonus(action, curr_metrics);
        reward += action_bonus;

        // Penalties for critical thresholds
        reward += self.calculate_threshold_penalties(curr_metrics);

        reward
    }

    fn calculate_performance_improvement(&self, prev: &PortageMetrics, curr: &PortageMetrics) -> f64 {
        // Simplified performance calculation
        let cpu_improvement = prev.system_metrics.cpu_usage_percent - curr.system_metrics.cpu_usage_percent;
        let memory_improvement = prev.system_metrics.memory_usage_percent - curr.system_metrics.memory_usage_percent;

        (cpu_improvement + memory_improvement) / 200.0  // Normalize to -1 to 1
    }

    fn calculate_resource_efficiency(&self, prev: &PortageMetrics, curr: &PortageMetrics) -> f64 {
        // Calculate improvement in resource usage
        let disk_efficiency = prev.system_metrics.disk_usage_percent - curr.system_metrics.disk_usage_percent;
        disk_efficiency / 100.0  // Normalize to -1 to 1
    }

    fn calculate_system_stability(&self, prev: &PortageMetrics, curr: &PortageMetrics) -> f64 {
        // Calculate system stability based on load averages and error rates
        let load_improvement = prev.system_metrics.load_average_1min - curr.system_metrics.load_average_1min;
        load_improvement / 10.0  // Normalize
    }

    fn calculate_time_saved(&self, _prev: &PortageMetrics, _curr: &PortageMetrics, action: &Action) -> f64 {
        // Simplified time calculation - would be more sophisticated in practice
        match action {
            Action::AdjustParallelism { .. } => 0.1,
            Action::OptimizeBuildOrder { .. } => 0.2,
            Action::ScheduleOperation { .. } => 0.15,
            _ => 0.0,
        }
    }

    fn calculate_action_bonus(&self, action: &Action, metrics: &PortageMetrics) -> f64 {
        match action {
            Action::CleanObsoletePackages { .. } => {
                // Bonus for cleaning when disk is full
                if metrics.system_metrics.disk_usage_percent > 80.0 {
                    0.5
                } else {
                    -0.1
                }
            },
            Action::AdjustParallelism { jobs } => {
                // Bonus for optimal parallelism adjustment
                if *jobs > 1 && *jobs <= 8 {
                    0.2
                } else {
                    0.0
                }
            },
            _ => 0.0,
        }
    }

    fn calculate_threshold_penalties(&self, metrics: &PortageMetrics) -> f64 {
        let mut penalty = 0.0;

        if metrics.system_metrics.cpu_usage_percent > 95.0 {
            penalty -= 0.5;
        }
        if metrics.system_metrics.memory_usage_percent > 90.0 {
            penalty -= 0.5;
        }
        if metrics.system_metrics.disk_usage_percent > 95.0 {
            penalty -= 1.0;
        }

        penalty
    }
}

pub struct TaskTracker {
    current_task: Option<TaskContext>,
    task_history: Vec<TaskContext>,
}

impl TaskTracker {
    pub fn new() -> Self {
        Self {
            current_task: None,
            task_history: Vec::new(),
        }
    }

    pub async fn update(&mut self, experience: Experience) -> Result<()> {
        // Update current task context based on experience
        // This is a simplified implementation
        Ok(())
    }

    pub fn get_current_task(&self) -> Option<&TaskContext> {
        self.current_task.as_ref()
    }
}