use anyhow::{Context, Result};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::Path;
use tracing::{debug, info, warn};

use crate::config::LearningConfig;
use crate::actions::Action;
use crate::SystemMetrics;

const STATE_SIZE: usize = 4;  // [disk_usage, free_space_trend, metadata_usage, fragmentation]

#[derive(Debug, Clone)]
pub struct State {
    pub features: Vec<f64>,
}

impl State {
    pub fn from_metrics(metrics: &SystemMetrics) -> Self {
        Self {
            features: vec![
                metrics.disk_usage_percent / 100.0,  // Normalize to 0-1
                metrics.free_space_mb / 10000.0,     // Normalize (10GB = 1.0)
                metrics.metadata_usage_percent / 100.0,
                metrics.fragmentation_percent / 100.0,
            ],
        }
    }
}

#[derive(Debug, Clone)]
struct Experience {
    state: State,
    action: Action,
    reward: f64,
    next_state: State,
    outcome_quality: f64, // 0-1, how good the outcome was
}

pub struct ReinforcementLearner {
    // Use a simpler approach without ML libraries that have complex type constraints
    replay_buffer: VecDeque<Experience>,
    config: LearningConfig,
    step_count: usize,
    epsilon: f64,
    action_history: Vec<(State, Action, f64)>, // (state, action, reward) history
    action_success_rates: Vec<f64>, // Success rate for each action type
}

impl ReinforcementLearner {
    pub fn new(config: &LearningConfig) -> Result<Self> {
        let mut learner = Self {
            replay_buffer: VecDeque::with_capacity(10000),
            config: config.clone(),
            step_count: 0,
            epsilon: config.exploration_rate,
            action_history: Vec::new(),
            action_success_rates: vec![0.5; Action::action_count()], // Initialize with neutral values
        };
        
        // Try to load existing model
        if let Err(e) = learner.load_model() {
            info!("No existing model found, starting fresh: {}", e);
        }
        
        Ok(learner)
    }
    
    pub fn select_action(&mut self, state: &State) -> Result<Action> {
        // Epsilon-greedy action selection
        if thread_rng().gen::<f64>() < self.epsilon {
            // Random exploration
            let action_id = thread_rng().gen_range(0..Action::action_count());
            debug!("Selected random action: {}", action_id);
            return Ok(Action::from_id(action_id).unwrap_or(Action::NoOperation));
        }
        
        // Use learned success rates combined with heuristics
        let action = self.select_best_action(state);
        debug!("Selected learned action: {:?} for state: {:?}", action, state.features);
        Ok(action)
    }
    
    pub fn select_best_action(&self, state: &State) -> Action {
        let disk_usage = state.features[0]; // 0-1 normalized
        let free_space = state.features[1];  // 0-1 normalized (10GB = 1.0)
        
        // Get candidate actions based on current state
        let candidate_actions = if disk_usage >= 0.98 { 
            // Emergency - only cleanup actions
            vec![Action::DeleteTempFiles, Action::CleanupSnapshots]
        } else if disk_usage >= 0.95 { 
            // Critical - prefer cleanup over maintenance
            vec![Action::DeleteTempFiles, Action::CompressFiles, Action::CleanupSnapshots]
        } else if disk_usage >= 0.85 { 
            // Warning - all actions available
            vec![
                Action::DeleteTempFiles, 
                Action::CompressFiles, 
                Action::BalanceMetadata,
                Action::CleanupSnapshots
            ]
        } else {
            // Normal - mostly no operation, occasional maintenance
            vec![Action::NoOperation, Action::BalanceMetadata, Action::CleanupSnapshots]
        };
        
        // Select action with highest success rate from candidates
        let mut best_action = Action::NoOperation;
        let mut best_score = 0.0;
        
        for action in candidate_actions {
            let action_idx = action as usize;
            let success_rate = self.action_success_rates[action_idx];
            
            // Add bonus for actions that are more appropriate for current state
            let context_bonus = match action {
                Action::DeleteTempFiles if disk_usage > 0.90 => 0.2,
                Action::CompressFiles if disk_usage > 0.85 => 0.1,
                Action::NoOperation if disk_usage < 0.80 => 0.3,
                _ => 0.0,
            };
            
            let total_score = success_rate + context_bonus;
            
            if total_score > best_score {
                best_score = total_score;
                best_action = action;
            }
        }
        
        best_action
    }
    
    pub fn update(&mut self, state: &State, action: Action, reward: f64, next_state: &State) -> Result<()> {
        // Calculate outcome quality based on the reward and state improvement
        let state_improvement = self.calculate_state_improvement(state, next_state);
        let outcome_quality = self.normalize_reward_to_quality(reward, state_improvement);
        
        // Store experience in replay buffer
        let experience = Experience {
            state: state.clone(),
            action,
            reward,
            next_state: next_state.clone(),
            outcome_quality,
        };
        
        self.replay_buffer.push_back(experience.clone());
        if self.replay_buffer.len() > 10000 {
            self.replay_buffer.pop_front();
        }
        
        // Add to action history for pattern analysis
        self.action_history.push((state.clone(), action, reward));
        if self.action_history.len() > 1000 {
            self.action_history.remove(0);
        }
        
        self.step_count += 1;
        
        // Decay epsilon
        self.epsilon = (self.config.exploration_rate * 0.995_f64.powi(self.step_count as i32))
            .max(0.01);
        
        // Update action success rates based on reward
        self.update_success_rates(action, reward);
        
        // Save model periodically
        if self.step_count % 100 == 0 {
            if let Err(e) = self.save_model() {
                warn!("Failed to save model: {}", e);
            }
        }
        
        debug!("Learning update complete. Step: {}, Epsilon: {:.3}, Buffer size: {}, Reward: {:.2}", 
               self.step_count, self.epsilon, self.replay_buffer.len(), reward);
        
        Ok(())
    }
    
    fn update_success_rates(&mut self, action: Action, reward: f64) {
        let action_idx = action as usize;
        let current_rate = self.action_success_rates[action_idx];
        
        // Convert reward to success indicator (1.0 for positive, 0.0 for negative)
        let success = if reward > 0.0 { 1.0 } else { 0.0 };
        
        // Update using exponential moving average
        let learning_rate = 0.1;
        self.action_success_rates[action_idx] = current_rate * (1.0 - learning_rate) + success * learning_rate;
        
        debug!("Updated success rate for {:?}: {:.3}", action, self.action_success_rates[action_idx]);
    }
    
    fn calculate_state_improvement(&self, prev_state: &State, curr_state: &State) -> f64 {
        // Calculate improvement in disk usage (lower is better)
        let usage_improvement = prev_state.features[0] - curr_state.features[0];
        
        // Calculate improvement in free space (higher is better)  
        let space_improvement = curr_state.features[1] - prev_state.features[1];
        
        // Combine improvements (weighted)
        (usage_improvement * 0.7) + (space_improvement * 0.3)
    }
    
    fn normalize_reward_to_quality(&self, reward: f64, state_improvement: f64) -> f64 {
        // Convert reward and state improvement to a quality score between 0 and 1
        let quality = if reward > 0.0 {
            0.5 + (reward / 100.0).min(0.5) // Cap positive rewards
        } else {
            0.5 + (reward / 100.0).max(-0.5) // Cap negative penalties
        };
        
        // Adjust by state improvement
        let adjusted_quality = quality + (state_improvement * 0.2);
        adjusted_quality.clamp(0.0, 1.0)
    }
    
    fn save_model(&self) -> Result<()> {
        let model_path = Path::new(&self.config.model_path);
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = model_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create model directory")?;
        }
        
        // Save model state information including success rates
        let model_info = ModelInfo {
            step_count: self.step_count,
            epsilon: self.epsilon,
            buffer_size: self.replay_buffer.len(),
            action_success_rates: self.action_success_rates.clone(),
        };
        
        let serialized = serde_json::to_string_pretty(&model_info)
            .context("Failed to serialize model info")?;
        
        std::fs::write(format!("{}.json", model_path.display()), serialized)
            .context("Failed to write model info")?;
        
        debug!("Model saved to {}", model_path.display());
        Ok(())
    }
    
    fn load_model(&mut self) -> Result<()> {
        let model_path = Path::new(&self.config.model_path);
        
        if !model_path.with_extension("json").exists() {
            return Err(anyhow::anyhow!("Model file does not exist"));
        }
        
        let content = std::fs::read_to_string(format!("{}.json", model_path.display()))
            .context("Failed to read model info")?;
        
        let model_info: ModelInfo = serde_json::from_str(&content)
            .context("Failed to deserialize model info")?;
        
        self.step_count = model_info.step_count;
        self.epsilon = model_info.epsilon;
        self.action_success_rates = model_info.action_success_rates;
        
        info!("Model loaded from {} (steps: {}, epsilon: {:.3})", 
              model_path.display(), self.step_count, self.epsilon);
        
        Ok(())
    }
    
    // Provide insights into learning progress
    pub fn get_learning_stats(&self) -> LearningStats {
        let avg_reward = if self.action_history.is_empty() {
            0.0
        } else {
            self.action_history.iter().map(|(_, _, r)| r).sum::<f64>() / self.action_history.len() as f64
        };
        
        let action_counts = self.count_actions();
        
        LearningStats {
            total_steps: self.step_count,
            exploration_rate: self.epsilon,
            buffer_size: self.replay_buffer.len(),
            average_reward: avg_reward,
            has_trained_model: self.step_count > 50, // Consider trained after some experience
            action_distribution: action_counts,
        }
    }
    
    fn count_actions(&self) -> Vec<(Action, usize)> {
        let mut counts = vec![
            (Action::NoOperation, 0),
            (Action::DeleteTempFiles, 0),
            (Action::CompressFiles, 0),
            (Action::BalanceMetadata, 0),
            (Action::CleanupSnapshots, 0),
        ];
        
        for (_, action, _) in &self.action_history {
            if let Some((_, count)) = counts.iter_mut().find(|(a, _)| a == action) {
                *count += 1;
            }
        }
        
        counts
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ModelInfo {
    step_count: usize,
    epsilon: f64,
    buffer_size: usize,
    action_success_rates: Vec<f64>,
}

#[derive(Debug)]
pub struct LearningStats {
    pub total_steps: usize,
    pub exploration_rate: f64,
    pub buffer_size: usize,
    pub average_reward: f64,
    pub has_trained_model: bool,
    pub action_distribution: Vec<(Action, usize)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SystemMetrics;
    use chrono::Utc;
    
    fn create_test_config() -> LearningConfig {
        LearningConfig {
            model_path: "/tmp/test_model".to_string(),
            model_update_interval: 3600,
            reward_smoothing: 0.95,
            exploration_rate: 0.1,
            learning_rate: 0.001,
            discount_factor: 0.99,
        }
    }
    
    fn create_test_metrics(usage: f64) -> SystemMetrics {
        SystemMetrics {
            timestamp: Utc::now(),
            disk_usage_percent: usage,
            free_space_mb: 1000.0,
            metadata_usage_percent: 5.0,
            fragmentation_percent: 10.0,
        }
    }
    
    #[test]
    fn test_state_creation() {
        let metrics = create_test_metrics(85.5);
        let state = State::from_metrics(&metrics);
        
        assert_eq!(state.features.len(), STATE_SIZE);
        assert!((state.features[0] - 0.855).abs() < 1e-6); // Normalized disk usage
    }
    
    #[tokio::test]
    async fn test_learner_creation() {
        let config = create_test_config();
        let learner = ReinforcementLearner::new(&config);
        assert!(learner.is_ok());
    }
    
    #[tokio::test]
    async fn test_action_selection() {
        let config = create_test_config();
        let mut learner = ReinforcementLearner::new(&config).unwrap();
        
        let metrics = create_test_metrics(75.0);
        let state = State::from_metrics(&metrics);
        
        let action = learner.select_action(&state);
        assert!(action.is_ok());
    }
    
    #[tokio::test]
    async fn test_heuristic_actions() {
        let config = create_test_config();
        let mut learner = ReinforcementLearner::new(&config).unwrap();
        
        // Test emergency threshold
        let emergency_state = State::from_metrics(&create_test_metrics(99.0));
        let action = learner.select_best_action(&emergency_state);
        assert_eq!(action, Action::DeleteTempFiles);
        
        // Test normal operation
        let normal_state = State::from_metrics(&create_test_metrics(70.0));
        let action = learner.select_best_action(&normal_state);
        assert_eq!(action, Action::NoOperation);
    }
    
    #[tokio::test]
    async fn test_learning_update() {
        let config = create_test_config();
        let mut learner = ReinforcementLearner::new(&config).unwrap();
        
        let state1 = State::from_metrics(&create_test_metrics(90.0));
        let state2 = State::from_metrics(&create_test_metrics(85.0));
        
        let result = learner.update(&state1, Action::DeleteTempFiles, 10.0, &state2);
        assert!(result.is_ok());
        
        // Check that experience was added to buffer
        assert_eq!(learner.replay_buffer.len(), 1);
        assert_eq!(learner.action_history.len(), 1);
    }
    
    #[test]
    fn test_state_improvement_calculation() {
        let config = create_test_config();
        let learner = ReinforcementLearner::new(&config).unwrap();
        
        let prev_state = State::from_metrics(&create_test_metrics(90.0));
        let curr_state = State::from_metrics(&create_test_metrics(85.0));
        
        let improvement = learner.calculate_state_improvement(&prev_state, &curr_state);
        assert!(improvement > 0.0); // Should be positive improvement
    }
    
    #[test]
    fn test_learning_stats() {
        let config = create_test_config();
        let mut learner = ReinforcementLearner::new(&config).unwrap();
        
        // Add some action history
        learner.action_history.push((
            State::from_metrics(&create_test_metrics(80.0)),
            Action::DeleteTempFiles,
            5.0
        ));
        
        let stats = learner.get_learning_stats();
        assert_eq!(stats.average_reward, 5.0);
        assert!(!stats.has_trained_model);
    }
}
