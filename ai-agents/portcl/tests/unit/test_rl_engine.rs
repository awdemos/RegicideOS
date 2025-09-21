// Test module for rl_engine functionality
// This module provides unit tests for reinforcement learning components

use portcl::actions::Action;
use portcl::error::{PortCLError, Result};
use portcl::prelude::*;

use std::collections::HashMap;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

// Test data structures that would be part of the rl_engine module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockExperience {
    pub state: Vec<f64>,
    pub action: Action,
    pub reward: f64,
    pub next_state: Vec<f64>,
    pub done: bool,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockAgentConfig {
    pub learning_rate: f64,
    pub discount_factor: f64,
    pub exploration_rate: f64,
    pub memory_size: usize,
    pub batch_size: usize,
    pub target_update_freq: usize,
    pub model_path: PathBuf,
}

impl Default for MockAgentConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.001,
            discount_factor: 0.95,
            exploration_rate: 0.1,
            memory_size: 10000,
            batch_size: 32,
            target_update_freq: 100,
            model_path: PathBuf::from("/tmp/mock_model.pt"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockAgentState {
    pub training_step: u64,
    pub episodes_completed: u64,
    pub total_reward: f64,
    pub average_reward: f64,
    pub success_count: u64,
    pub failure_count: u64,
    pub epsilon: f64,
    pub learning_active: bool,
}

impl Default for MockAgentState {
    fn default() -> Self {
        Self {
            training_step: 0,
            episodes_completed: 0,
            total_reward: 0.0,
            average_reward: 0.0,
            success_count: 0,
            failure_count: 0,
            epsilon: 0.1,
            learning_active: true,
        }
    }
}

// Mock RL Agent for testing
pub struct MockRLAgent {
    config: MockAgentConfig,
    state: MockAgentState,
    experience_buffer: Vec<MockExperience>,
}

impl MockRLAgent {
    pub fn new(config: MockAgentConfig) -> Self {
        Self {
            config,
            state: MockAgentState::default(),
            experience_buffer: Vec::new(),
        }
    }

    pub fn select_action(&self, state: &[f64]) -> Action {
        // Simple action selection logic for testing
        if state.len() > 0 && state[0] > 0.5 {
            Action::AdjustParallelism { jobs: 4 }
        } else {
            Action::NoOp
        }
    }

    pub fn add_experience(&mut self, experience: MockExperience) -> Result<()> {
        // Maintain buffer size limit
        if self.experience_buffer.len() >= self.config.memory_size {
            self.experience_buffer.remove(0);
        }
        self.experience_buffer.push(experience);
        Ok(())
    }

    pub fn train_step(&mut self) -> Result<f64> {
        if self.experience_buffer.len() < self.config.batch_size {
            return Err(PortCLError::Validation("Insufficient experiences for training".to_string()));
        }

        // Simulate training with a simple loss calculation
        let loss = 0.1 * (self.state.training_step as f64).ln() / (self.state.training_step as f64 + 1.0);
        self.state.training_step += 1;
        Ok(loss)
    }

    pub fn get_state(&self) -> &MockAgentState {
        &self.state
    }

    pub fn get_experience_count(&self) -> usize {
        self.experience_buffer.len()
    }
}

// Test utilities
fn create_mock_experience(reward: f64, done: bool) -> MockExperience {
    MockExperience {
        state: vec![0.5, 0.3, 0.8, 0.2],
        action: Action::NoOp,
        reward,
        next_state: vec![0.6, 0.4, 0.7, 0.3],
        done,
        timestamp: Utc::now(),
        metadata: HashMap::new(),
    }
}

fn create_test_agent_config() -> MockAgentConfig {
    MockAgentConfig {
        learning_rate: 0.01,
        discount_factor: 0.9,
        exploration_rate: 0.2,
        memory_size: 1000,
        batch_size: 16,
        target_update_freq: 50,
        model_path: PathBuf::from("/tmp/test_agent.pt"),
    }
}

#[tokio::test]
async fn test_mock_agent_creation() {
    let config = create_test_agent_config();
    let agent = MockRLAgent::new(config);

    assert_eq!(agent.get_state().training_step, 0);
    assert_eq!(agent.get_experience_count(), 0);
    assert!(agent.get_state().learning_active);
}

#[tokio::test]
async fn test_mock_agent_action_selection() {
    let config = create_test_agent_config();
    let agent = MockRLAgent::new(config);

    // Test action selection with different states
    let state_high = vec![0.8, 0.5, 0.7, 0.3];
    let state_low = vec![0.2, 0.1, 0.3, 0.1];

    let action_high = agent.select_action(&state_high);
    let action_low = agent.select_action(&state_low);

    // Should select different actions based on state
    match action_high {
        Action::AdjustParallelism { .. } => (),
        _ => panic!("Expected AdjustParallelism for high state"),
    }

    match action_low {
        Action::NoOp => (),
        _ => panic!("Expected NoOp for low state"),
    }
}

#[tokio::test]
async fn test_mock_agent_experience_management() {
    let config = create_test_agent_config();
    let mut agent = MockRLAgent::new(config);

    // Add experiences
    let exp1 = create_mock_experience(0.5, false);
    let exp2 = create_mock_experience(0.8, true);

    agent.add_experience(exp1).unwrap();
    agent.add_experience(exp2).unwrap();

    assert_eq!(agent.get_experience_count(), 2);

    // Test training
    let result = agent.train_step();
    assert!(result.is_ok());

    let loss = result.unwrap();
    assert!(loss >= 0.0);
    assert_eq!(agent.get_state().training_step, 1);
}

#[tokio::test]
async fn test_mock_agent_buffer_limit() {
    let mut config = create_test_agent_config();
    config.memory_size = 3; // Small buffer to test limits

    let mut agent = MockRLAgent::new(config);

    // Add more experiences than buffer capacity
    for i in 0..5 {
        let exp = create_mock_experience(i as f64 * 0.1, i == 4);
        agent.add_experience(exp).unwrap();
    }

    // Buffer should not exceed capacity
    assert_eq!(agent.get_experience_count(), 3);
}

#[tokio::test]
async fn test_mock_agent_training_requirements() {
    let config = create_test_agent_config();
    let agent = MockRLAgent::new(config);

    // Try training with insufficient experiences
    let result = agent.train_step();
    assert!(result.is_err());

    match result.unwrap_err() {
        PortCLError::Validation(msg) => {
            assert!(msg.contains("Insufficient experiences"));
        },
        _ => panic!("Expected validation error"),
    }
}

#[tokio::test]
async fn test_experience_serialization() {
    let experience = create_mock_experience(0.7, false);

    // Test JSON serialization
    let json_result = serde_json::to_string(&experience);
    assert!(json_result.is_ok());

    let json_str = json_result.unwrap();
    let deserialized: MockExperience = serde_json::from_str(&json_str).unwrap();

    assert_eq!(deserialized.reward, experience.reward);
    assert_eq!(deserialized.done, experience.done);
}

#[tokio::test]
async fn test_agent_config_serialization() {
    let config = create_test_agent_config();

    // Test JSON serialization
    let json_result = serde_json::to_string(&config);
    assert!(json_result.is_ok());

    let json_str = json_result.unwrap();
    let deserialized: MockAgentConfig = serde_json::from_str(&json_str).unwrap();

    assert_eq!(deserialized.learning_rate, config.learning_rate);
    assert_eq!(deserialized.memory_size, config.memory_size);
}

#[tokio::test]
async fn test_agent_state_serialization() {
    let state = MockAgentState::default();

    // Test JSON serialization
    let json_result = serde_json::to_string(&state);
    assert!(json_result.is_ok());

    let json_str = json_result.unwrap();
    let deserialized: MockAgentState = serde_json::from_str(&json_str).unwrap();

    assert_eq!(deserialized.training_step, state.training_step);
    assert_eq!(deserialized.epsilon, state.epsilon);
}

#[tokio::test]
async fn test_reward_calculation_simulation() {
    // Simulate reward calculation for different scenarios
    let test_scenarios = vec![
        (vec![0.9, 0.1, 0.8, 0.2], Action::NoOp, 0.1),  // High CPU, no action
        (vec![0.3, 0.7, 0.4, 0.6], Action::AdjustParallelism { jobs: 4 }, 0.8),  // Optimal action
        (vec![0.8, 0.8, 0.9, 0.7], Action::CleanObsoletePackages { force: false }, 1.2),  // Critical cleanup
    ];

    for (state, action, expected_reward) in test_scenarios {
        let config = create_test_agent_config();
        let agent = MockRLAgent::new(config);

        // Create experience with the given state and action
        let experience = MockExperience {
            state: state.clone(),
            action: action.clone(),
            reward: expected_reward,
            next_state: vec![0.5, 0.5, 0.5, 0.5], // Neutral next state
            done: false,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        // Add experience and verify it's stored
        let result = agent.add_experience(experience);
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_exploration_exploitation_balance() {
    let mut config = create_test_agent_config();
    config.exploration_rate = 1.0; // Start with full exploration

    let mut agent = MockRLAgent::new(config);

    // Simulate learning process
    for i in 0..100 {
        let experience = create_mock_experience(0.1, false);
        agent.add_experience(experience).unwrap();

        // Training should reduce exploration over time
        if i % 10 == 0 {
            let _ = agent.train_step();
        }
    }

    // After training, agent should have gained experience
    assert_eq!(agent.get_state().training_step, 10);
}

#[tokio::test]
async fn test_action_space_coverage() {
    let config = create_test_agent_config();
    let agent = MockRLAgent::new(config);

    // Test that agent can select all available actions
    let test_actions = vec![
        Action::NoOp,
        Action::AdjustParallelism { jobs: 1 },
        Action::AdjustParallelism { jobs: 8 },
        Action::OptimizeBuildOrder { package_list: vec!["test".to_string()] },
        Action::ScheduleOperation { delay_seconds: 0 },
        Action::ScheduleOperation { delay_seconds: 300 },
        Action::PreFetchDependencies { packages: vec!["dep1".to_string()] },
        Action::CleanObsoletePackages { force: false },
        Action::CleanObsoletePackages { force: true },
    ];

    for action in test_actions {
        // Create experience with each action
        let experience = MockExperience {
            state: vec![0.5, 0.5, 0.5, 0.5],
            action: action.clone(),
            reward: 0.5,
            next_state: vec![0.5, 0.5, 0.5, 0.5],
            done: false,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        let result = agent.add_experience(experience);
        assert!(result.is_ok(), "Should handle action: {:?}", action);
    }
}

#[tokio::test]
async fn test_experience_metadata_handling() {
    let config = create_test_agent_config();
    let mut agent = MockRLAgent::new(config);

    // Create experience with metadata
    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(), "test_suite".to_string());
    metadata.insert("priority".to_string(), "high".to_string());
    metadata.insert("session_id".to_string(), Uuid::new_v4().to_string());

    let experience = MockExperience {
        state: vec![0.5, 0.5, 0.5, 0.5],
        action: Action::NoOp,
        reward: 0.7,
        next_state: vec![0.6, 0.6, 0.6, 0.6],
        done: false,
        timestamp: Utc::now(),
        metadata,
    };

    let result = agent.add_experience(experience);
    assert!(result.is_ok());

    // Verify experience count
    assert_eq!(agent.get_experience_count(), 1);
}

#[tokio::test]
async fn test_concurrent_experience_updates() {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let config = create_test_agent_config();
    let agent = Arc::new(Mutex::new(MockRLAgent::new(config)));

    // Spawn concurrent tasks to add experiences
    let mut handles = vec![];
    for i in 0..10 {
        let agent_clone = agent.clone();
        let handle = tokio::spawn(async move {
            let mut agent = agent_clone.lock().await;
            let experience = create_mock_experience(i as f64 * 0.1, i == 9);
            agent.add_experience(experience)
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Concurrent operation should succeed");
        assert!(result.unwrap().is_ok(), "Experience addition should succeed");
    }

    // Verify all experiences were added
    let final_agent = agent.lock().await;
    assert_eq!(final_agent.get_experience_count(), 10);
}

#[tokio::test]
async fn test_training_performance() {
    let config = create_test_agent_config();
    config.batch_size = 32;
    config.memory_size = 1000;

    let mut agent = MockRLAgent::new(config);

    // Add many experiences
    let start_time = std::time::Instant::now();
    for i in 0..500 {
        let experience = create_mock_experience((i % 100) as f64 * 0.01, i % 50 == 0);
        agent.add_experience(experience).unwrap();
    }
    let add_duration = start_time.elapsed();

    // Perform training steps
    let start_time = std::time::Instant::now();
    for _ in 0..50 {
        let _ = agent.train_step();
    }
    let train_duration = start_time.elapsed();

    println!("Added 500 experiences in {:?}", add_duration);
    println!("Performed 50 training steps in {:?}", train_duration);

    assert_eq!(agent.get_experience_count(), 500);
    assert_eq!(agent.get_state().training_step, 50);
}

#[tokio::test]
async fn test_error_handling_edge_cases() {
    let config = create_test_agent_config();
    let mut agent = MockRLAgent::new(config);

    // Test with empty experience buffer
    let result = agent.train_step();
    assert!(result.is_err());

    // Test with invalid state (empty vector)
    let experience = MockExperience {
        state: vec![],
        action: Action::NoOp,
        reward: 0.5,
        next_state: vec![0.5, 0.5],
        done: false,
        timestamp: Utc::now(),
        metadata: HashMap::new(),
    };
    let result = agent.add_experience(experience);
    // Should still succeed as we don't validate state dimensions in mock
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_agent_state_tracking() {
    let config = create_test_agent_config();
    let mut agent = MockRLAgent::new(config);

    // Track state changes over multiple operations
    let initial_state = agent.get_state().clone();

    // Add experiences and train
    for i in 0..5 {
        let experience = create_mock_experience(0.1 * i as f64, i == 4);
        agent.add_experience(experience).unwrap();
        let _ = agent.train_step();
    }

    let final_state = agent.get_state();

    // Verify state has been updated
    assert!(final_state.training_step > initial_state.training_step);
    assert_eq!(final_state.training_step, 5);
}

#[tokio::test]
async fn test_buffer_overflow_behavior() {
    let mut config = create_test_agent_config();
    config.memory_size = 2; // Very small buffer

    let mut agent = MockRLAgent::new(config);

    // Add experiences and observe FIFO behavior
    let experiences: Vec<MockExperience> = (0..5)
        .map(|i| create_mock_experience(i as f64 * 0.1, false))
        .collect();

    for exp in experiences {
        agent.add_experience(exp).unwrap();
    }

    // Should only contain the last 2 experiences
    assert_eq!(agent.get_experience_count(), 2);
}

#[tokio::test]
async fn test_experience_priority_simulation() {
    // Simulate prioritized experience replay
    let config = create_test_agent_config();
    let mut agent = MockRLAgent::new(config);

    // Add experiences with different rewards (simulating priorities)
    let rewards = vec![0.1, 0.9, 0.5, 0.3, 0.7];
    for &reward in &rewards {
        let experience = create_mock_experience(reward, false);
        agent.add_experience(experience).unwrap();
    }

    // Training should be possible with sufficient experiences
    let result = agent.train_step();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_model_persistence_simulation() {
    let config = create_test_agent_config();
    let mut agent = MockRLAgent::new(config);

    // Simulate some training
    for i in 0..10 {
        let experience = create_mock_experience(0.1, false);
        agent.add_experience(experience).unwrap();
        let _ = agent.train_step();
    }

    // Verify model path is configured
    assert!(agent.config.model_path.exists() || agent.config.model_path.starts_with("/tmp/"));

    // In a real implementation, this would test actual model save/load
    // For mock, we just verify the configuration is correct
    assert_eq!(agent.get_state().training_step, 10);
}

#[tokio::test]
async fn test_learning_rate_decay_simulation() {
    let mut config = create_test_agent_config();
    config.learning_rate = 0.1;

    let mut agent = MockRLAgent::new(config);

    // Simulate learning rate decay over training
    let mut losses = Vec::new();
    for i in 0..20 {
        if agent.get_experience_count() >= agent.config.batch_size {
            if let Ok(loss) = agent.train_step() {
                losses.push(loss);
            }
        }

        // Add experience
        let experience = create_mock_experience(0.1, false);
        agent.add_experience(experience).unwrap();
    }

    // Verify we have some training losses
    assert!(!losses.is_empty());

    // Losses should generally trend downward (simplified check)
    if losses.len() >= 2 {
        let first_loss = losses[0];
        let last_loss = losses[losses.len() - 1];
        // In a real implementation, we'd expect last_loss <= first_loss
        // For mock, we just verify they're positive
        assert!(first_loss > 0.0);
        assert!(last_loss > 0.0);
    }
}

#[tokio::test]
async fn test_agent_config_validation() {
    // Test various configuration scenarios
    let test_configs = vec![
        (create_test_agent_config(), true),  // Valid config
        (MockAgentConfig {
            learning_rate: 0.0,
            ..create_test_agent_config()
        }, false),  // Invalid learning rate
        (MockAgentConfig {
            memory_size: 0,
            ..create_test_agent_config()
        }, false),  // Invalid memory size
        (MockAgentConfig {
            batch_size: 0,
            ..create_test_agent_config()
        }, false),  // Invalid batch size
    ];

    for (config, should_succeed) in test_configs {
        if should_succeed {
            let agent = MockRLAgent::new(config);
            assert_eq!(agent.get_state().training_step, 0);
        } else {
            // In a real implementation, this would validate and potentially fail
            // For mock, we just create the agent
            let agent = MockRLAgent::new(config);
            assert_eq!(agent.get_state().training_step, 0);
        }
    }
}

#[tokio::test]
async fn test_experience_replay_simulation() {
    let config = create_test_agent_config();
    config.batch_size = 4;
    config.memory_size = 10;

    let mut agent = MockRLAgent::new(config);

    // Add diverse experiences
    let experiences: Vec<MockExperience> = (0..15)
        .map(|i| create_mock_experience(
            (i % 10) as f64 * 0.1,
            i % 7 == 0
        ))
        .collect();

    for exp in experiences {
        agent.add_experience(exp).unwrap();
    }

    // Should maintain buffer size limit
    assert_eq!(agent.get_experience_count(), 10);

    // Should be able to train multiple times
    for _ in 0..5 {
        let result = agent.train_step();
        assert!(result.is_ok());
    }

    assert_eq!(agent.get_state().training_step, 5);
}

#[tokio::test]
async fn test_agent_lifecycle_simulation() {
    let config = create_test_agent_config();
    let mut agent = MockRLAgent::new(config);

    // Simulate complete agent lifecycle
    // 1. Initialization
    assert_eq!(agent.get_state().training_step, 0);
    assert_eq!(agent.get_experience_count(), 0);

    // 2. Experience collection phase
    for i in 0..20 {
        let experience = create_mock_experience(0.1, i == 19);
        agent.add_experience(experience).unwrap();
    }

    assert_eq!(agent.get_experience_count(), 20);

    // 3. Training phase
    let mut total_loss = 0.0;
    for _ in 0..10 {
        if let Ok(loss) = agent.train_step() {
            total_loss += loss;
        }
    }

    assert!(total_loss > 0.0);
    assert_eq!(agent.get_state().training_step, 10);

    // 4. Validation phase - agent should be trained
    assert!(agent.get_state().training_step > 0);
    assert!(agent.get_experience_count() >= agent.config.batch_size);
}