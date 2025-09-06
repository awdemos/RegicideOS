use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time;
use tracing::{info, warn, error, debug};

mod btrfs;
mod learning;
mod actions;
mod config;

use btrfs::BtrfsMonitor;
use learning::{ReinforcementLearner, State};
use actions::{ActionExecutor, Action};
use config::Config;

#[derive(Parser)]
#[command(name = "btrmind")]
#[command(about = "AI-powered BTRFS storage monitoring and optimization")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    #[arg(short, long, default_value = "/etc/btrmind/config.toml")]
    config: PathBuf,
    
    #[arg(short, long)]
    dry_run: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the monitoring daemon
    Run,
    /// Analyze current storage state
    Analyze,
    /// Run cleanup actions manually
    Cleanup {
        #[arg(long)]
        aggressive: bool,
    },
    /// Display current statistics
    Stats,
    /// Validate configuration
    Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub disk_usage_percent: f64,
    pub free_space_mb: f64,
    pub metadata_usage_percent: f64,
    pub fragmentation_percent: f64,
}

pub struct BtrMindAgent {
    monitor: BtrfsMonitor,
    pub learner: ReinforcementLearner,
    executor: ActionExecutor,
    config: Config,
    last_metrics: Option<SystemMetrics>,
}

impl BtrMindAgent {
    pub fn new(config: Config) -> Result<Self> {
        let monitor = BtrfsMonitor::new(&config.monitoring.target_path)?;
        let learner = ReinforcementLearner::new(&config.learning)?;
        let executor = ActionExecutor::new(config.actions.clone(), config.dry_run);
        
        Ok(Self {
            monitor,
            learner,
            executor,
            config,
            last_metrics: None,
        })
    }
    
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting BtrMind agent");
        info!("Target path: {}", self.config.monitoring.target_path);
        info!("Poll interval: {}s", self.config.monitoring.poll_interval);
        
        let mut interval = time::interval(Duration::from_secs(self.config.monitoring.poll_interval));
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.monitoring_cycle().await {
                error!("Monitoring cycle failed: {}", e);
                // Continue running despite errors
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    }
    
    async fn monitoring_cycle(&mut self) -> Result<()> {
        // 1. Observe current state
        let metrics = self.monitor.collect_metrics().await?;
        debug!("Collected metrics: {:?}", metrics);
        
        // 2. Convert to ML state representation
        let state = State::from_metrics(&metrics);
        
        // 3. Get action from RL agent
        let action = self.learner.select_action(&state)?;
        debug!("Selected action: {:?}", action);
        
        // 4. Execute action
        let action_result = self.executor.execute_action(action).await;
        
        // 5. Calculate reward
        let reward = if let Some(ref prev_metrics) = self.last_metrics {
            self.calculate_reward(prev_metrics, &metrics)
        } else {
            0.0 // No reward for first observation
        };
        
        // 6. Update learning model
        if let Some(ref prev_metrics) = self.last_metrics {
            let prev_state = State::from_metrics(prev_metrics);
            self.learner.update(&prev_state, action, reward, &state)?;
        }
        
        // 7. Log and alert if needed
        self.check_thresholds(&metrics).await?;
        
        // 8. Store metrics for next cycle
        self.last_metrics = Some(metrics);
        
        if action_result.is_err() {
            warn!("Action execution failed: {:?}", action_result);
        }
        
        Ok(())
    }
    
    fn calculate_reward(&self, prev_metrics: &SystemMetrics, curr_metrics: &SystemMetrics) -> f64 {
        let util_delta = prev_metrics.disk_usage_percent - curr_metrics.disk_usage_percent;
        
        // Base reward: positive if space freed
        let mut reward = util_delta * 10.0;
        
        // Penalties for critical thresholds
        if curr_metrics.disk_usage_percent > self.config.thresholds.critical_level {
            reward -= 50.0; // Severe penalty
        } else if curr_metrics.disk_usage_percent > self.config.thresholds.warning_level {
            reward -= 15.0; // Moderate penalty
        }
        
        // Bonus for sustained improvement
        if util_delta > 2.0 {
            reward += 5.0;
        }
        
        debug!("Reward calculation: util_delta={:.2}, reward={:.2}", util_delta, reward);
        reward
    }
    
    async fn check_thresholds(&self, metrics: &SystemMetrics) -> Result<()> {
        if metrics.disk_usage_percent >= self.config.thresholds.emergency_level {
            error!("EMERGENCY: Disk usage at {:.1}%! Immediate action required!", 
                  metrics.disk_usage_percent);
            // TODO: Send system notification
        } else if metrics.disk_usage_percent >= self.config.thresholds.critical_level {
            warn!("CRITICAL: Disk usage at {:.1}%", metrics.disk_usage_percent);
        } else if metrics.disk_usage_percent >= self.config.thresholds.warning_level {
            info!("WARNING: Disk usage at {:.1}%", metrics.disk_usage_percent);
        }
        
        Ok(())
    }
    
    pub async fn analyze(&self) -> Result<()> {
        let metrics = self.monitor.collect_metrics().await?;
        
        println!("=== BtrMind Storage Analysis ===");
        println!("Timestamp: {}", metrics.timestamp.format("%Y-%m-%d %H:%M:%S UTC"));
        println!("Disk Usage: {:.1}%", metrics.disk_usage_percent);
        println!("Free Space: {:.1} MB", metrics.free_space_mb);
        println!("Metadata Usage: {:.1}%", metrics.metadata_usage_percent);
        println!("Fragmentation: {:.1}%", metrics.fragmentation_percent);
        
        // Threshold status
        if metrics.disk_usage_percent >= self.config.thresholds.emergency_level {
            println!("Status: 🔴 EMERGENCY");
        } else if metrics.disk_usage_percent >= self.config.thresholds.critical_level {
            println!("Status: 🟠 CRITICAL");
        } else if metrics.disk_usage_percent >= self.config.thresholds.warning_level {
            println!("Status: 🟡 WARNING");
        } else {
            println!("Status: 🟢 NORMAL");
        }
        
        Ok(())
    }
    
    pub async fn cleanup(&mut self, aggressive: bool) -> Result<()> {
        info!("Running manual cleanup (aggressive: {})", aggressive);
        
        if aggressive {
            // Run all cleanup actions
            for action in [Action::DeleteTempFiles, Action::CompressFiles, 
                          Action::BalanceMetadata, Action::CleanupSnapshots] {
                info!("Executing action: {:?}", action);
                if let Err(e) = self.executor.execute_action(action).await {
                    warn!("Action failed: {:?}", e);
                }
            }
        } else {
            // Run safe cleanup only
            for action in [Action::DeleteTempFiles, Action::CleanupSnapshots] {
                info!("Executing action: {:?}", action);
                if let Err(e) = self.executor.execute_action(action).await {
                    warn!("Action failed: {:?}", e);
                }
            }
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    let cli = Cli::parse();
    
    // Load configuration
    let config = Config::load(&cli.config)
        .with_context(|| format!("Failed to load config from {:?}", cli.config))?;
    
    // Override dry_run from CLI
    let mut config = config;
    if cli.dry_run {
        config.dry_run = true;
        info!("Running in DRY-RUN mode - no actions will be executed");
    }
    
    let mut agent = BtrMindAgent::new(config)?;
    
    match cli.command {
        Some(Commands::Run) | None => {
            agent.run().await?;
        },
        Some(Commands::Analyze) => {
            agent.analyze().await?;
        },
        Some(Commands::Cleanup { aggressive }) => {
            agent.cleanup(aggressive).await?;
        },
        Some(Commands::Stats) => {
            let stats = agent.learner.get_learning_stats();
            println!("=== BtrMind Learning Statistics ===");
            println!("Total Steps: {}", stats.total_steps);
            println!("Exploration Rate: {:.3}", stats.exploration_rate);
            println!("Experience Buffer Size: {}", stats.buffer_size);
            println!("Average Reward: {:.2}", stats.average_reward);
            println!("Has Trained Model: {}", stats.has_trained_model);
            println!("\nAction Distribution:");
            for (action, count) in stats.action_distribution {
                println!("  {:?}: {} times", action, count);
            }
        },
        Some(Commands::Config) => {
            println!("Configuration validation:");
            println!("Config file: {:?}", cli.config);
            println!("✓ Configuration loaded successfully");
        },
    }
    
    Ok(())
}
