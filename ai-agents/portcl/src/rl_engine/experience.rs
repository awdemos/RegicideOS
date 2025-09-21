use crate::error::{PortCLError, Result};
use crate::rl_engine::model::Experience;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use rand::seq::SliceRandom;
use rand::thread_rng;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceBuffer {
    pub capacity: usize,
    pub experiences: VecDeque<Experience>,
    pub priorities: Vec<f64>,
    pub alpha: f64,  // Prioritization exponent
    pub beta: f64,   // Importance sampling exponent
    pub beta_increment: f64,
    pub max_priority: f64,
}

impl ExperienceBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            experiences: VecDeque::with_capacity(capacity),
            priorities: Vec::with_capacity(capacity),
            alpha: 0.6,  // Prioritization exponent
            beta: 0.4,   // Importance sampling exponent
            beta_increment: 0.001,
            max_priority: 1.0,
        }
    }

    pub fn add(&mut self, experience: Experience, priority: Option<f64>) {
        let priority = priority.unwrap_or(self.max_priority);

        if self.experiences.len() >= self.capacity {
            self.experiences.pop_front();
            self.priorities.remove(0);
        }

        self.experiences.push_back(experience);
        self.priorities.push(priority);

        debug!("Added experience to buffer. Size: {}/{}", self.experiences.len(), self.capacity);
    }

    pub fn sample(&self, batch_size: usize) -> Result<(Vec<Experience>, Vec<f64>, Vec<usize>)> {
        if self.experiences.is_empty() {
            return Err(PortCLError::RLEngine("Experience buffer is empty".to_string()));
        }

        if batch_size > self.experiences.len() {
            warn!("Requested batch size {} is larger than buffer size {}", batch_size, self.experiences.len());
        }

        let actual_batch_size = batch_size.min(self.experiences.len());

        // Calculate sampling probabilities
        let mut probs: Vec<f64> = self.priorities.iter()
            .map(|&p| p.powf(self.alpha))
            .collect();

        let sum_probs: f64 = probs.iter().sum();
        if sum_probs > 0.0 {
            for prob in &mut probs {
                *prob /= sum_probs;
            }
        }

        // Sample experiences based on probabilities
        let mut rng = thread_rng();
        let mut indices: Vec<usize> = (0..self.experiences.len()).collect();
        indices.shuffle(&mut rng);
        indices.truncate(actual_batch_size);

        let experiences: Vec<Experience> = indices.iter()
            .map(|&i| self.experiences[i].clone())
            .collect();

        let importance_weights: Vec<f64> = indices.iter()
            .map(|&i| {
                let prob = probs[i];
                if prob > 0.0 {
                    (self.experiences.len() as f64 * prob).powf(-self.beta)
                } else {
                    1.0
                }
            })
            .collect();

        // Normalize importance weights
        if let Some(max_weight) = importance_weights.iter().fold(None, |acc, &w| {
            Some(acc.map_or(w, |a: f64| a.max(w)))
        }) {
            if max_weight > 0.0 {
                // Note: normalization would happen here, but we return raw weights for now
            }
        }

        debug!("Sampled {} experiences from buffer", actual_batch_size);

        Ok((experiences, importance_weights, indices))
    }

    pub fn uniform_sample(&self, batch_size: usize) -> Result<Vec<Experience>> {
        if self.experiences.is_empty() {
            return Err(PortCLError::RLEngine("Experience buffer is empty".to_string()));
        }

        let actual_batch_size = batch_size.min(self.experiences.len());
        let mut rng = thread_rng();

        let mut experiences: Vec<Experience> = self.experiences.iter()
            .cloned()
            .collect();
        experiences.shuffle(&mut rng);
        experiences.truncate(actual_batch_size);

        Ok(experiences)
    }

    pub fn update_priorities(&mut self, indices: &[usize], priorities: &[f64]) -> Result<()> {
        if indices.len() != priorities.len() {
            return Err(PortCLError::RLEngine(
                "Indices and priorities length mismatch".to_string()
            ));
        }

        for (&idx, &priority) in indices.iter().zip(priorities.iter()) {
            if idx < self.priorities.len() {
                self.priorities[idx] = priority.abs() + 1e-6;  // Small epsilon to avoid zero
                self.max_priority = self.max_priority.max(self.priorities[idx]);
            }
        }

        debug!("Updated priorities for {} experiences", indices.len());
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.experiences.len()
    }

    pub fn is_empty(&self) -> bool {
        self.experiences.is_empty()
    }

    pub fn clear(&mut self) {
        self.experiences.clear();
        self.priorities.clear();
        self.max_priority = 1.0;
        info!("Experience buffer cleared");
    }

    pub fn update_beta(&mut self) {
        self.beta = (self.beta + self.beta_increment).min(1.0);
    }

    pub fn get_statistics(&self) -> ExperienceBufferStats {
        let avg_priority = if self.priorities.is_empty() {
            0.0
        } else {
            self.priorities.iter().sum::<f64>() / self.priorities.len() as f64
        };

        ExperienceBufferStats {
            size: self.experiences.len(),
            capacity: self.capacity,
            avg_priority,
            max_priority: self.max_priority,
            alpha: self.alpha,
            beta: self.beta,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceBufferStats {
    pub size: usize,
    pub capacity: usize,
    pub avg_priority: f64,
    pub max_priority: f64,
    pub alpha: f64,
    pub beta: f64,
}

impl Default for ExperienceBufferStats {
    fn default() -> Self {
        Self {
            size: 0,
            capacity: 0,
            avg_priority: 0.0,
            max_priority: 0.0,
            alpha: 0.6,
            beta: 0.4,
        }
    }
}

pub struct ReplayBuffer {
    buffer: Arc<Mutex<ExperienceBuffer>>,
    config: ReplayBufferConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayBufferConfig {
    pub capacity: usize,
    pub use_prioritized_replay: bool,
    pub alpha: f64,
    pub beta_start: f64,
    pub beta_increment: f64,
}

impl Default for ReplayBufferConfig {
    fn default() -> Self {
        Self {
            capacity: 10000,
            use_prioritized_replay: true,
            alpha: 0.6,
            beta_start: 0.4,
            beta_increment: 0.001,
        }
    }
}

impl ReplayBuffer {
    pub fn new(config: ReplayBufferConfig) -> Self {
        let mut buffer = ExperienceBuffer::new(config.capacity);
        buffer.alpha = config.alpha;
        buffer.beta = config.beta_start;
        buffer.beta_increment = config.beta_increment;

        Self {
            buffer: Arc::new(Mutex::new(buffer)),
            config,
        }
    }

    pub fn add_experience(&self, experience: Experience, priority: Option<f64>) -> Result<()> {
        let mut buffer = self.buffer.lock()
            .map_err(|_| PortCLError::RLEngine("Failed to lock experience buffer".to_string()))?;

        buffer.add(experience, priority);

        // Update beta for importance sampling
        buffer.update_beta();

        Ok(())
    }

    pub fn sample_batch(&self, batch_size: usize) -> Result<(Vec<Experience>, Vec<f64>, Vec<usize>)> {
        let buffer = self.buffer.lock()
            .map_err(|_| PortCLError::RLEngine("Failed to lock experience buffer".to_string()))?;

        if self.config.use_prioritized_replay {
            buffer.sample(batch_size)
        } else {
            let experiences = buffer.uniform_sample(batch_size)?;
            let weights = vec![1.0; experiences.len()];
            let indices = (0..experiences.len()).collect();
            Ok((experiences, weights, indices))
        }
    }

    pub fn update_priorities(&self, indices: &[usize], priorities: &[f64]) -> Result<()> {
        if !self.config.use_prioritized_replay {
            return Ok(());
        }

        let mut buffer = self.buffer.lock()
            .map_err(|_| PortCLError::RLEngine("Failed to lock experience buffer".to_string()))?;

        buffer.update_priorities(indices, priorities)
    }

    pub fn len(&self) -> usize {
        let buffer = self.buffer.lock()
            .map_err(|_| PortCLError::RLEngine("Failed to lock experience buffer".to_string()))
            .unwrap();

        buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        let buffer = self.buffer.lock()
            .map_err(|_| PortCLError::RLEngine("Failed to lock experience buffer".to_string()))
            .unwrap();

        buffer.is_empty()
    }

    pub fn clear(&self) -> Result<()> {
        let mut buffer = self.buffer.lock()
            .map_err(|_| PortCLError::RLEngine("Failed to lock experience buffer".to_string()))?;

        buffer.clear();
        Ok(())
    }

    pub fn get_statistics(&self) -> Result<ExperienceBufferStats> {
        let buffer = self.buffer.lock()
            .map_err(|_| PortCLError::RLEngine("Failed to lock experience buffer".to_string()))?;

        Ok(buffer.get_statistics())
    }

    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let buffer = self.buffer.lock()
            .map_err(|_| PortCLError::RLEngine("Failed to lock experience buffer".to_string()))?;

        let data = serde_json::to_string_pretty(&*buffer)
            .map_err(|e| PortCLError::Json(e))?;

        std::fs::write(path, data)
            .map_err(|e| PortCLError::Io(e))?;

        info!("Experience buffer saved to {}", path);
        Ok(())
    }

    pub fn load_from_file(&self, path: &str) -> Result<()> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| PortCLError::Io(e))?;

        let loaded_buffer: ExperienceBuffer = serde_json::from_str(&data)
            .map_err(|e| PortCLError::Json(e))?;

        let mut buffer = self.buffer.lock()
            .map_err(|_| PortCLError::RLEngine("Failed to lock experience buffer".to_string()))?;

        *buffer = loaded_buffer;

        info!("Experience buffer loaded from {}", path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_experience_buffer() {
        let mut buffer = ExperienceBuffer::new(100);

        // Add some experiences
        let experience = Experience::new(
            ndarray::Array1::zeros(10),
            crate::actions::Action::NoOp,
            1.0,
            ndarray::Array1::zeros(10),
            false,
        );

        buffer.add(experience.clone(), None);
        assert_eq!(buffer.len(), 1);

        // Test sampling
        let (sampled, _, _) = buffer.sample(1).unwrap();
        assert_eq!(sampled.len(), 1);

        // Test priority update
        buffer.update_priorities(&[0], &[2.0]).unwrap();
        assert_eq!(buffer.priorities[0], 2.0);
    }

    #[test]
    fn test_replay_buffer() {
        let config = ReplayBufferConfig::default();
        let replay_buffer = ReplayBuffer::new(config);

        let experience = Experience::new(
            ndarray::Array1::zeros(10),
            crate::actions::Action::NoOp,
            1.0,
            ndarray::Array1::zeros(10),
            false,
        );

        replay_buffer.add_experience(experience, None).unwrap();
        assert_eq!(replay_buffer.len(), 1);

        let stats = replay_buffer.get_statistics().unwrap();
        assert_eq!(stats.size, 1);
    }
}