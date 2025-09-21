//! Mock PortageAgent for testing PortCL reinforcement learning
//!
//! This module provides a mock implementation of PortageAgent that
//! simulates reinforcement learning operations without requiring
//! actual neural network computations or model training.

use crate::fixtures::mock_data::*;
use crate::fixtures::mock_monitor::*;
use crate::error::PortCLError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ndarray::Array1;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::Mutex;

/// Mock experience tuple for reinforcement learning
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MockExperience {
    pub state: Vec<f64>,
    pub action: MockAction,
    pub reward: f64,
    pub next_state: Vec<f64>,
    pub done: bool,
    pub timestamp: DateTime<Utc>,
    pub episode_id: String,
}

/// Mock learning policy
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MockLearningPolicy {
    pub policy_type: PolicyType,
    pub epsilon: f64,
    pub epsilon_decay: f64,
    pub min_epsilon: f64,
    pub q_values: HashMap<String, f64>,
    pub action_preferences: HashMap<String, f64>,
}

/// Policy type for action selection
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum PolicyType {
    EpsilonGreedy,
    UCB,
    ThompsonSampling,
    Softmax,
}

/// Mock model state
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MockModelState {
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
    pub model_accuracy: f64,
    pub convergence_rate: f64,
}

/// Mock training statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MockTrainingStats {
    pub total_episodes: u64,
    pub successful_episodes: u64,
    pub average_episode_length: f64,
    pub average_reward_per_episode: f64,
    pub convergence_achieved: bool,
    pub best_performance: f64,
    pub learning_curve: Vec<(u64, f64)>, // (episode, reward)
    pub exploration_exploitation_balance: f64,
}

/// Mock task context for continual learning
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MockTaskContext {
    pub task_id: String,
    pub task_name: String,
    pub task_type: String,
    pub performance_metrics: HashMap<String, f64>,
    pub learning_progress: f64,
    pub expertise_level: f64,
    pub last_active: DateTime<Utc>,
}

/// Mock PortageAgent for testing reinforcement learning
#[derive(Debug, Clone)]
pub struct MockPortageAgent {
    config: MockRLConfig,
    state: Arc<RwLock<MockModelState>>,
    policy: Arc<RwLock<MockLearningPolicy>>,
    experience_buffer: Arc<Mutex<Vec<MockExperience>>>,
    training_stats: Arc<RwLock<MockTrainingStats>>,
    task_contexts: Arc<RwLock<HashMap<String, MockTaskContext>>>,
    error_injection: Arc<RwLock<HashMap<String, bool>>>,
    delay_injection: Arc<RwLock<HashMap<String, u64>>>,
    learning_mode: Arc<RwLock<bool>>,
}

impl MockPortageAgent {
    /// Create a new mock PortageAgent with default configuration
    pub fn new(config: MockRLConfig) -> Self {
        let initial_policy = MockLearningPolicy {
            policy_type: PolicyType::EpsilonGreedy,
            epsilon: config.exploration_rate,
            epsilon_decay: config.exploration_decay,
            min_epsilon: 0.01,
            q_values: HashMap::new(),
            action_preferences: HashMap::new(),
        };

        Self {
            config,
            state: Arc::new(RwLock::new(MockModelState::default())),
            policy: Arc::new(RwLock::new(initial_policy)),
            experience_buffer: Arc::new(Mutex::new(Vec::new())),
            training_stats: Arc::new(RwLock::new(MockTrainingStats::default())),
            task_contexts: Arc::new(RwLock::new(HashMap::new())),
            error_injection: Arc::new(RwLock::new(HashMap::new())),
            delay_injection: Arc::new(RwLock::new(HashMap::new())),
            learning_mode: Arc::new(RwLock::new(true)),
        }
    }

    /// Create a mock agent with pre-trained knowledge
    pub fn with_pretrained_knowledge(config: MockRLConfig, knowledge: HashMap<String, f64>) -> Self {
        let agent = Self::new(config);

        // Initialize Q-values with provided knowledge
        let mut policy = agent.policy.write().unwrap();
        for (action_key, q_value) in knowledge {
            policy.q_values.insert(action_key, q_value);
        }

        agent
    }

    /// Inject an error for a specific operation
    pub fn inject_error(&self, operation: String) {
        let mut errors = self.error_injection.write().unwrap();
        errors.insert(operation, true);
    }

    /// Inject a delay for a specific operation (in milliseconds)
    pub fn inject_delay(&self, operation: String, delay_ms: u64) {
        let mut delays = self.delay_injection.write().unwrap();
        delays.insert(operation, delay_ms);
    }

    /// Clear all injected errors and delays
    pub fn clear_injections(&self) {
        let mut errors = self.error_injection.write().unwrap();
        let mut delays = self.delay_injection.write().unwrap();
        errors.clear();
        delays.clear();
    }

    /// Set learning mode (enable/disable)
    pub fn set_learning_mode(&self, enabled: bool) {
        let mut mode = self.learning_mode.write().unwrap();
        *mode = enabled;
    }

    /// Get current model state
    pub fn get_state(&self) -> MockModelState {
        self.state.read().unwrap().clone()
    }

    /// Get training statistics
    pub fn get_training_stats(&self) -> MockTrainingStats {
        self.training_stats.read().unwrap().clone()
    }

    /// Get experience buffer
    pub async fn get_experience_buffer(&self) -> Vec<MockExperience> {
        self.experience_buffer.lock().await.clone()
    }

    /// Get task contexts
    pub fn get_task_contexts(&self) -> HashMap<String, MockTaskContext> {
        self.task_contexts.read().unwrap().clone()
    }

    /// Add a task context for continual learning
    pub fn add_task_context(&self, context: MockTaskContext) {
        let mut contexts = self.task_contexts.write().unwrap();
        contexts.insert(context.task_id.clone(), context);
    }

    /// Simulate learning progress
    pub fn simulate_learning_progress(&self, episodes: u64, success_rate: f64) {
        let mut state = self.state.write().unwrap();
        let mut stats = self.training_stats.write().unwrap();

        for _ in 0..episodes {
            state.episodes_completed += 1;
            state.total_reward += success_rate * 10.0;
            state.average_reward = state.total_reward / state.episodes_completed as f64;

            if success_rate > 0.5 {
                state.success_count += 1;
            } else {
                state.failure_count += 1;
            }

            // Simulate decreasing exploration
            let mut policy = self.policy.write().unwrap();
            policy.epsilon = (policy.epsilon * policy.epsilon_decay).max(policy.min_epsilon);

            // Update learning curve
            stats.learning_curve.push((state.episodes_completed, success_rate * 10.0));
        }

        stats.total_episodes = state.episodes_completed;
        stats.successful_episodes = state.success_count;
        stats.average_reward_per_episode = state.average_reward;
        stats.convergence_achieved = state.episodes_completed > 100 && success_rate > 0.8;
    }

    /// Check if an operation should fail based on injected errors
    fn should_fail(&self, operation: &str) -> bool {
        let errors = self.error_injection.read().unwrap();
        errors.get(operation).copied().unwrap_or(false)
    }

    /// Get delay for an operation based on injected delays
    fn get_delay(&self, operation: &str) -> u64 {
        let delays = self.delay_injection.read().unwrap();
        delays.get(operation).copied().unwrap_or(0)
    }

    /// Simulate async delay
    async fn simulate_delay(&self, operation: &str) {
        let delay = self.get_delay(operation);
        if delay > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
        }
    }

    /// Convert system metrics to state representation
    fn metrics_to_state(&self, metrics: &SystemMetrics) -> Vec<f64> {
        vec![
            metrics.cpu_usage / 100.0,     // Normalize to [0, 1]
            metrics.memory_usage / 100.0, // Normalize to [0, 1]
            metrics.disk_usage / 100.0,    // Normalize to [0, 1]
            (metrics.load_average.get(0).unwrap_or(&0.0) / 4.0).min(1.0), // Normalize load average
            (metrics.active_connections as f64 / 100.0).min(1.0), // Normalize connections
        ]
    }

    /// Calculate mock Q-value for an action
    fn calculate_q_value(&self, state: &[f64], action: &MockAction) -> f64 {
        let policy = self.policy.read().unwrap();
        let action_key = format!("{:?}", action.action_type());

        // Base Q-value from learned knowledge
        let mut q_value = policy.q_values.get(&action_key).copied().unwrap_or(0.0);

        // Adjust based on current state
        if state[0] > 0.8 { // High CPU usage
            match action {
                MockAction::AdjustParallelism { .. } => q_value += 2.0,
                MockAction::CleanObsoletePackages { .. } => q_value -= 1.0,
                _ => {}
            }
        }

        if state[1] > 0.8 { // High memory usage
            match action {
                MockAction::CleanObsoletePackages { .. } => q_value += 1.5,
                MockAction::NoOp => q_value += 0.5,
                _ => {}
            }
        }

        q_value
    }

    /// Select action using epsilon-greedy policy
    fn select_action_epsilon_greedy(&self, state: &[f64]) -> MockAction {
        let policy = self.policy.read().unwrap();

        if rand::random::<f64>() < policy.epsilon {
            // Exploration: select random action
            let actions = sample_actions();
            actions[rand::random::<usize>() % actions.len()].clone()
        } else {
            // Exploitation: select best action
            let actions = sample_actions();
            let mut best_action = actions[0].clone();
            let mut best_q_value = self.calculate_q_value(state, &best_action);

            for action in &actions[1..] {
                let q_value = self.calculate_q_value(state, action);
                if q_value > best_q_value {
                    best_q_value = q_value;
                    best_action = action.clone();
                }
            }

            best_action
        }
    }

    /// Update Q-values based on experience
    fn update_q_value(&self, experience: &MockExperience) {
        let mut policy = self.policy.write().unwrap();
        let action_key = format!("{:?}", experience.action.action_type());

        // Q-learning update: Q(s,a) = Q(s,a) + α[r + γ*max(Q(s',a')) - Q(s,a)]
        let current_q = policy.q_values.get(&action_key).copied().unwrap_or(0.0);
        let learning_rate = self.config.learning_rate;
        let discount_factor = self.config.discount_factor;

        // Find max Q-value for next state
        let actions = sample_actions();
        let max_next_q = actions.iter()
            .map(|action| self.calculate_q_value(&experience.next_state, action))
            .fold(0.0, f64::max);

        let new_q = current_q + learning_rate * (experience.reward + discount_factor * max_next_q - current_q);
        policy.q_values.insert(action_key, new_q);
    }

    /// Simulate continual learning adaptation
    fn adapt_to_task_context(&self, context: &MockTaskContext) {
        let mut policy = self.policy.write().unwrap();

        // Adjust action preferences based on task performance
        for (action, performance) in &context.performance_metrics {
            if performance > 0.7 { // Good performance
                *policy.action_preferences.entry(action.clone()).or_insert(0.5) += 0.1;
            } else if performance < 0.3 { // Poor performance
                *policy.action_preferences.entry(action.clone()).or_insert(0.5) -= 0.1;
            }
        }

        // Ensure preferences stay in reasonable bounds
        for preference in policy.action_preferences.values_mut() {
            *preference = preference.clamp(0.0, 1.0);
        }
    }
}

#[async_trait]
pub trait MockPortageAgentTrait {
    async fn select_action(&self, metrics: &SystemMetrics) -> Result<MockAction, PortCLError>;
    async fn update_experience(&self, experience: MockExperience) -> Result<(), PortCLError>;
    async fn train_step(&self) -> Result<f64, PortCLError>;
    async fn save_model(&self) -> Result<(), PortCLError>;
    async fn load_model(&self) -> Result<(), PortCLError>;
    async fn get_agent_state(&self) -> MockModelState;
    async fn switch_task(&self, task_id: String) -> Result<(), PortCLError>;
    async fn evaluate_performance(&self, test_episodes: u32) -> Result<f64, PortCLError>;
    async fn continual_learning_step(&self) -> Result<(), PortCLError>;
}

#[async_trait]
impl MockPortageAgentTrait for MockPortageAgent {
    async fn select_action(&self, metrics: &SystemMetrics) -> Result<MockAction, PortCLError> {
        self.simulate_delay("select_action").await;

        if self.should_fail("select_action") {
            return Err(PortCLError::RLEngine("Mock error: Failed to select action".to_string()));
        }

        let state = self.metrics_to_state(metrics);
        let action = self.select_action_epsilon_greedy(&state);

        // Update state
        let mut agent_state = self.state.write().unwrap();
        agent_state.current_episode_steps += 1;

        Ok(action)
    }

    async fn update_experience(&self, experience: MockExperience) -> Result<(), PortCLError> {
        self.simulate_delay("update_experience").await;

        if self.should_fail("update_experience") {
            return Err(PortCLError::RLEngine("Mock error: Failed to update experience".to_string()));
        }

        // Add to experience buffer
        let mut buffer = self.experience_buffer.lock().await;
        buffer.push(experience.clone());

        // Limit buffer size
        if buffer.len() > self.config.memory_size {
            buffer.remove(0);
        }

        // Update Q-values
        self.update_q_value(&experience);

        // Update agent state
        let mut state = self.state.write().unwrap();
        state.total_reward += experience.reward;
        state.average_reward = state.total_reward / state.episodes_completed as f64;

        // Check if episode completed
        if experience.done {
            state.episodes_completed += 1;
            state.current_episode_steps = 0;

            if experience.reward > 0.0 {
                state.success_count += 1;
            } else {
                state.failure_count += 1;
            }
        }

        // Update policy epsilon
        let mut policy = self.policy.write().unwrap();
        policy.epsilon = (policy.epsilon * policy.epsilon_decay).max(policy.min_epsilon);
        state.epsilon = policy.epsilon;

        Ok(())
    }

    async fn train_step(&self) -> Result<f64, PortCLError> {
        self.simulate_delay("train_step").await;

        if self.should_fail("train_step") {
            return Err(PortCLError::RLEngine("Mock error: Training step failed".to_string()));
        }

        let learning_enabled = *self.learning_mode.read().unwrap();
        if !learning_enabled {
            return Ok(0.0);
        }

        let buffer = self.experience_buffer.lock().await;
        if buffer.len() < self.config.batch_size {
            return Ok(0.0);
        }

        // Simulate training loss
        let mock_loss = 0.1 + rand::random::<f64>() * 0.05;

        // Update training stats
        let mut state = self.state.write().unwrap();
        state.training_step += 1;
        state.last_training_loss = mock_loss;

        // Simulate model improvement
        state.model_accuracy = (state.model_accuracy + 0.001).min(0.95);
        state.convergence_rate = (state.convergence_rate + 0.0001).min(0.1);

        Ok(mock_loss)
    }

    async fn save_model(&self) -> Result<(), PortCLError> {
        self.simulate_delay("save_model").await;

        if self.should_fail("save_model") {
            return Err(PortCLError::RLEngine("Mock error: Failed to save model".to_string()));
        }

        // Simulate model saving
        let state = self.state.read().unwrap();
        println!("Mock model saved at training step: {}, accuracy: {:.3}",
                 state.training_step, state.model_accuracy);

        Ok(())
    }

    async fn load_model(&self) -> Result<(), PortCLError> {
        self.simulate_delay("load_model").await;

        if self.should_fail("load_model") {
            return Err(PortCLError::RLEngine("Mock error: Failed to load model".to_string()));
        }

        // Simulate model loading
        let mut state = self.state.write().unwrap();
        state.model_accuracy = 0.85; // Loaded model has good accuracy
        state.convergence_rate = 0.05;

        Ok(())
    }

    async fn get_agent_state(&self) -> MockModelState {
        self.state.read().unwrap().clone()
    }

    async fn switch_task(&self, task_id: String) -> Result<(), PortCLError> {
        let contexts = self.task_contexts.read().unwrap();

        if let Some(context) = contexts.get(&task_id) {
            self.adapt_to_task_context(context);
            Ok(())
        } else {
            Err(PortCLError::NotFound(format!("Task context {} not found", task_id)))
        }
    }

    async fn evaluate_performance(&self, test_episodes: u32) -> Result<f64, PortCLError> {
        self.simulate_delay("evaluate_performance").await;

        if self.should_fail("evaluate_performance") {
            return Err(PortCLError::RLEngine("Mock error: Performance evaluation failed".to_string()));
        }

        // Simulate performance evaluation
        let state = self.state.read().unwrap();
        let base_performance = state.model_accuracy;
        let variation = (rand::random::<f64>() - 0.5) * 0.1;
        let performance = (base_performance + variation).clamp(0.0, 1.0);

        Ok(performance)
    }

    async fn continual_learning_step(&self) -> Result<(), PortCLError> {
        self.simulate_delay("continual_learning_step").await;

        if self.should_fail("continual_learning_step") {
            return Err(PortCLError::RLEngine("Mock error: Continual learning step failed".to_string()));
        }

        if !self.config.enable_continual_learning {
            return Ok(());
        }

        // Simulate knowledge consolidation
        let mut policy = self.policy.write().unwrap();

        // Consolidate Q-values (prevent forgetting)
        for q_value in policy.q_values.values_mut() {
            *q_value = *q_value * 0.95 + 0.05 * 0.5; // Gentle consolidation
        }

        Ok(())
    }
}

impl Default for MockModelState {
    fn default() -> Self {
        Self {
            training_step: 0,
            episodes_completed: 0,
            total_reward: 0.0,
            average_reward: 0.0,
            success_count: 0,
            failure_count: 0,
            current_episode_steps: 0,
            last_training_loss: 0.0,
            epsilon: 0.1,
            learning_active: true,
            model_accuracy: 0.1, // Start with low accuracy
            convergence_rate: 0.01,
        }
    }
}

impl Default for MockTrainingStats {
    fn default() -> Self {
        Self {
            total_episodes: 0,
            successful_episodes: 0,
            average_episode_length: 0.0,
            average_reward_per_episode: 0.0,
            convergence_achieved: false,
            best_performance: 0.0,
            learning_curve: Vec::new(),
            exploration_exploitation_balance: 0.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_agent_creation() {
        let config = MockRLConfig::default();
        let agent = MockPortageAgent::new(config);

        let state = agent.get_state();
        assert_eq!(state.episodes_completed, 0);
        assert!(state.learning_active);
    }

    #[tokio::test]
    async fn test_action_selection() {
        let config = MockRLConfig::default();
        let agent = MockPortageAgent::new(config);

        let metrics = SystemMetrics {
            cpu_usage: 50.0,
            memory_usage: 60.0,
            disk_usage: 40.0,
            network_io: 1024,
            uptime: 3600,
            load_average: vec![1.0, 1.1, 1.2],
            process_count: 100,
            active_connections: 5,
        };

        let action = agent.select_action(&metrics).await.unwrap();
        // Should return some action
        assert!(matches!(action, MockAction::NoOp | MockAction::AdjustParallelism { .. }));
    }

    #[tokio::test]
    async fn test_experience_update() {
        let config = MockRLConfig::default();
        let agent = MockPortageAgent::new(config);

        let experience = MockExperience {
            state: vec![0.5, 0.6, 0.4, 0.3, 0.2],
            action: MockAction::NoOp,
            reward: 1.0,
            next_state: vec![0.4, 0.5, 0.3, 0.2, 0.1],
            done: true,
            timestamp: Utc::now(),
            episode_id: "test_episode".to_string(),
        };

        agent.update_experience(experience).await.unwrap();

        let state = agent.get_state();
        assert_eq!(state.episodes_completed, 1);
        assert_eq!(state.total_reward, 1.0);
        assert_eq!(state.success_count, 1);
    }

    #[tokio::test]
    async fn test_error_injection() {
        let config = MockRLConfig::default();
        let agent = MockPortageAgent::new(config);

        agent.inject_error("select_action".to_string());

        let metrics = SystemMetrics {
            cpu_usage: 50.0,
            memory_usage: 60.0,
            disk_usage: 40.0,
            network_io: 1024,
            uptime: 3600,
            load_average: vec![1.0],
            process_count: 100,
            active_connections: 5,
        };

        let result = agent.select_action(&metrics).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_learning_progress_simulation() {
        let config = MockRLConfig::default();
        let agent = MockPortageAgent::new(config);

        agent.simulate_learning_progress(100, 0.8);

        let state = agent.get_state();
        assert_eq!(state.episodes_completed, 100);
        assert!(state.total_reward > 0.0);
        assert!(state.success_count > 0);

        let stats = agent.get_training_stats();
        assert_eq!(stats.total_episodes, 100);
    }

    #[tokio::test]
    async fn test_continual_learning() {
        let config = MockRLConfig::default();
        config.enable_continual_learning = true;
        let agent = MockPortageAgent::new(config);

        let task_context = MockTaskContext {
            task_id: "task_1".to_string(),
            task_name: "Package Optimization".to_string(),
            task_type: "optimization".to_string(),
            performance_metrics: {
                let mut metrics = HashMap::new();
                metrics.insert("AdjustParallelism".to_string(), 0.9);
                metrics.insert("OptimizeBuildOrder".to_string(), 0.8);
                metrics
            },
            learning_progress: 0.7,
            expertise_level: 0.6,
            last_active: Utc::now(),
        };

        agent.add_task_context(task_context);
        agent.switch_task("task_1".to_string()).await.unwrap();

        let policy = agent.policy.read().unwrap();
        assert!(policy.action_preferences.contains_key("AdjustParallelism"));
    }
}