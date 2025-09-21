pub mod agent;
pub mod model;
pub mod experience;
pub mod continual;

pub use agent::{PortageAgent, AgentConfig};
pub use model::{DQNModel, ModelConfig};
pub use experience::{ExperienceBuffer, ReplayBuffer};
pub use continual::{ContinualLearning, ContinualLearningConfig};

use crate::config::RLConfig;
use crate::error::Result;
use crate::monitor::PortageMetrics;
use crate::actions::Action;
use crate::rl_engine::model::Experience;
use std::sync::{Arc, Mutex};

pub struct RLManager {
    config: RLConfig,
    agent: PortageAgent,
    continual_learning: Arc<Mutex<ContinualLearning>>,
}

impl RLManager {
    pub fn new(config: RLConfig) -> Result<Self> {
        let agent = PortageAgent::new(config.clone())?;
            let cl_config = ContinualLearningConfig {
            enable_ewc: config.enable_continual_learning,
            ewc_importance: 0.5,
            enable_progressive_networks: true,
            enable_policy_reuse: true,
            consolidation_threshold: 0.8,
            memory_retention_rate: 0.9,
            max_policies: 10,
            consolidation_interval: 100,
        };
        let continual_learning = Arc::new(Mutex::new(ContinualLearning::new(cl_config)?));

        Ok(Self {
            config,
            agent,
            continual_learning,
        })
    }

    pub async fn select_action(&self, metrics: &PortageMetrics) -> Result<Action> {
        self.agent.select_action(metrics).await
    }

    pub async fn update_experience(&mut self, experience: Experience) -> Result<()> {
        self.agent.update_experience(experience).await?;
        let mut cl = self.continual_learning.lock().unwrap();
        cl.consolidate_knowledge().await
    }

    pub async fn train_model(&self) -> Result<()> {
        self.agent.train_step().await?;
        Ok(())
    }

    pub async fn save_model(&self) -> Result<()> {
        self.agent.save_model().await
    }

    pub async fn load_model(&self) -> Result<()> {
        self.agent.load_model().await
    }
}